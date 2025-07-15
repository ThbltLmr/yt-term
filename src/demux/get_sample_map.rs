use std::{collections::VecDeque, error::Error, usize};

use super::get_moov_box::{MOOVBox, STCOBox, STSCBox, STSZBox, Streams};

pub type SampleMap = VecDeque<SampleData>;

#[derive(Clone, Debug)]
pub struct SampleData {
    pub size: u32,
    pub is_video: bool,
}

#[derive(Debug)]
pub struct ChunkData {
    pub is_video: bool,
    pub offset: u32,
    pub sample_sizes: Vec<u32>,
}

struct ChunkToSample {
    pub starting_chunk: u32,
    pub sample_count: u32,
}

pub fn get_sample_map(moov_box: MOOVBox) -> Result<SampleMap, Box<dyn Error>> {
    let mut chunk_data: VecDeque<ChunkData> = VecDeque::new();
    for trak in moov_box.traks {
        let chunk_offsets = parse_stco(&trak.media.minf.stbl.stco);
        let chunk_offsets_with_sample_count = parse_stsc(&trak.media.minf.stbl.stsc, chunk_offsets);

        match trak.media.minf.header {
            Streams::Audio => {
                let mut new_chunk_data = parse_stsz(
                    &trak.media.minf.stbl.stsz,
                    chunk_offsets_with_sample_count,
                    false,
                );

                chunk_data.append(&mut new_chunk_data);
            }
            Streams::Video => {
                let mut new_chunk_data = parse_stsz(
                    &trak.media.minf.stbl.stsz,
                    chunk_offsets_with_sample_count,
                    true,
                );

                chunk_data.append(&mut new_chunk_data);
            }
        }
    }

    Ok(format_sample_data(chunk_data))
}

fn format_sample_data(chunk_data: VecDeque<ChunkData>) -> VecDeque<SampleData> {
    let mut chunk_data_vec: Vec<ChunkData> = chunk_data.into();
    chunk_data_vec.sort_by(|a, b| a.offset.cmp(&b.offset));
    assert!(chunk_data_vec[0].offset < chunk_data_vec[1].offset);

    let mut sample_offsets_sum = 0;

    chunk_data_vec
        .iter()
        .flat_map(|chunk| {
            chunk
                .sample_sizes
                .iter()
                .map(|size| {
                    let sample_data: SampleData = SampleData {
                        size: *size,
                        is_video: chunk.is_video,
                    };

                    sample_offsets_sum += *size;
                    sample_data
                })
                .collect::<VecDeque<SampleData>>()
        })
        .collect::<VecDeque<SampleData>>()
}

fn parse_stco(stco_box: &STCOBox) -> Vec<u32> {
    let data: Vec<u32> = stco_box
        .data
        .chunks_exact(4)
        .map(|chunk| {
            let bytes: [u8; 4] = chunk.try_into().unwrap();
            u32::from_be_bytes(bytes)
        })
        .collect();

    let size = data[1];

    assert_eq!(size as usize, data.len() - 2);

    data[2..].to_vec()
}

fn parse_stsc(stsc: &STSCBox, chunk_offsets: Vec<u32>) -> Vec<(u32, u32)> {
    let data: Vec<u32> = stsc
        .data
        .chunks_exact(4)
        .map(|chunk| {
            let bytes: [u8; 4] = chunk.try_into().unwrap();
            u32::from_be_bytes(bytes)
        })
        .collect();

    let chunk_to_sample: Vec<ChunkToSample> = data[2..]
        .chunks_exact(3)
        .map(|chunk| ChunkToSample {
            starting_chunk: chunk[0],
            sample_count: chunk[1],
        })
        .collect();

    let mut result: Vec<(u32, u32)> = vec![];
    let mut current_sample_count_index = 0;
    let mut next_sample_count_index = 1;

    for (chunk_index, chunk_offset) in chunk_offsets.iter().enumerate() {
        if chunk_to_sample.len() > next_sample_count_index {
            if chunk_index + 1 >= chunk_to_sample[next_sample_count_index].starting_chunk as usize {
                current_sample_count_index = next_sample_count_index;
                next_sample_count_index += 1;
            }
        }
        let tuple = (
            chunk_offset.clone(),
            chunk_to_sample[current_sample_count_index].sample_count,
        );

        result.push(tuple);
    }

    result
}

fn parse_stsz(
    stsz: &STSZBox,
    chunk_offsets_with_sample_count: Vec<(u32, u32)>,
    is_video: bool,
) -> VecDeque<ChunkData> {
    let data: Vec<u32> = stsz
        .data
        .chunks_exact(4)
        .map(|chunk| {
            let bytes: [u8; 4] = chunk.try_into().unwrap();
            u32::from_be_bytes(bytes)
        })
        .collect();

    let general_size = data[1];

    if general_size != 0 {
        return chunk_offsets_with_sample_count
            .iter()
            .map(|(offset, sample_count)| ChunkData {
                is_video,
                offset: offset.clone(),
                sample_sizes: vec![general_size; sample_count.clone() as usize],
            })
            .collect();
    }

    let sizes: Vec<u32> = data[3..].to_vec();
    let mut current_index = 0;

    chunk_offsets_with_sample_count
        .iter()
        .map(|(offset, sample_count)| {
            let chunk_data = ChunkData {
                is_video,
                offset: offset.clone(),
                sample_sizes: sizes[current_index..(current_index + *sample_count as usize)]
                    .to_vec(),
            };
            current_index += *sample_count as usize;

            chunk_data
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::demux::get_moov_box::*;

    #[test]
    fn test_sample_data_creation() {
        let sample = SampleData {
            size: 1024,
            is_video: true,
        };
        
        assert_eq!(sample.size, 1024);
        assert!(sample.is_video);
    }

    #[test]
    fn test_chunk_data_creation() {
        let chunk = ChunkData {
            is_video: false,
            offset: 2048,
            sample_sizes: vec![512, 1024, 768],
        };
        
        assert!(!chunk.is_video);
        assert_eq!(chunk.offset, 2048);
        assert_eq!(chunk.sample_sizes.len(), 3);
        assert_eq!(chunk.sample_sizes[0], 512);
    }

    #[test]
    fn test_parse_stco() {
        let stco_box = STCOBox {
            size: 24,
            data: vec![
                0x00, 0x00, 0x00, 0x00, // version + flags
                0x00, 0x00, 0x00, 0x02, // entry count = 2
                0x00, 0x00, 0x10, 0x00, // offset 1 = 4096
                0x00, 0x00, 0x20, 0x00, // offset 2 = 8192
            ],
        };
        
        let offsets = parse_stco(&stco_box);
        assert_eq!(offsets.len(), 2);
        assert_eq!(offsets[0], 4096);
        assert_eq!(offsets[1], 8192);
    }

    #[test]
    fn test_parse_stsc() {
        let stsc_box = STSCBox {
            size: 28,
            data: vec![
                0x00, 0x00, 0x00, 0x00, // version + flags
                0x00, 0x00, 0x00, 0x01, // entry count = 1
                0x00, 0x00, 0x00, 0x01, // first chunk = 1
                0x00, 0x00, 0x00, 0x02, // samples per chunk = 2
                0x00, 0x00, 0x00, 0x01, // sample description index = 1
            ],
        };
        
        let chunk_offsets = vec![4096, 8192];
        let result = parse_stsc(&stsc_box, chunk_offsets);
        
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], (4096, 2)); // offset, sample count
        assert_eq!(result[1], (8192, 2));
    }

    #[test]
    fn test_parse_stsz_with_general_size() {
        let stsz_box = STSZBox {
            size: 20,
            data: vec![
                0x00, 0x00, 0x00, 0x00, // version + flags
                0x00, 0x00, 0x04, 0x00, // general size = 1024
                0x00, 0x00, 0x00, 0x02, // sample count = 2
            ],
        };
        
        let chunk_offsets = vec![(4096, 2)]; // offset, sample count
        let result = parse_stsz(&stsz_box, chunk_offsets, true);
        
        assert_eq!(result.len(), 1);
        let chunk = &result[0];
        assert!(chunk.is_video);
        assert_eq!(chunk.offset, 4096);
        assert_eq!(chunk.sample_sizes.len(), 2);
        assert_eq!(chunk.sample_sizes[0], 1024);
        assert_eq!(chunk.sample_sizes[1], 1024);
    }

    #[test]
    fn test_parse_stsz_with_individual_sizes() {
        let stsz_box = STSZBox {
            size: 28,
            data: vec![
                0x00, 0x00, 0x00, 0x00, // version + flags
                0x00, 0x00, 0x00, 0x00, // general size = 0 (use individual sizes)
                0x00, 0x00, 0x00, 0x02, // sample count = 2
                0x00, 0x00, 0x02, 0x00, // sample 1 size = 512
                0x00, 0x00, 0x04, 0x00, // sample 2 size = 1024
            ],
        };
        
        let chunk_offsets = vec![(4096, 2)]; // offset, sample count
        let result = parse_stsz(&stsz_box, chunk_offsets, false);
        
        assert_eq!(result.len(), 1);
        let chunk = &result[0];
        assert!(!chunk.is_video);
        assert_eq!(chunk.offset, 4096);
        assert_eq!(chunk.sample_sizes.len(), 2);
        assert_eq!(chunk.sample_sizes[0], 512);
        assert_eq!(chunk.sample_sizes[1], 1024);
    }

    #[test]
    fn test_format_sample_data() {
        let mut chunk_data = VecDeque::new();
        
        // Add chunks in non-sequential order to test sorting
        chunk_data.push_back(ChunkData {
            is_video: false,
            offset: 8192,
            sample_sizes: vec![256, 512],
        });
        
        chunk_data.push_back(ChunkData {
            is_video: true,
            offset: 4096,
            sample_sizes: vec![1024],
        });
        
        let sample_data = format_sample_data(chunk_data);
        
        assert_eq!(sample_data.len(), 3);
        
        // Should be sorted by offset, so video (4096) comes first
        assert!(sample_data[0].is_video);
        assert_eq!(sample_data[0].size, 1024);
        
        // Then audio samples (8192)
        assert!(!sample_data[1].is_video);
        assert_eq!(sample_data[1].size, 256);
        assert!(!sample_data[2].is_video);
        assert_eq!(sample_data[2].size, 512);
    }
}
