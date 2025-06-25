use std::{collections::VecDeque, error::Error, usize};

use super::moov::{MOOVBox, STCOBox, STSCBox, STSZBox, Streams};

pub type SampleData = (u32, bool);

#[derive(Debug)]
pub struct ChunkData {
    pub is_video: bool,
    pub offset: u32,
    pub sample_count: u32,
    pub sample_sizes: Vec<u32>,
}

struct ChunkToSample {
    pub starting_chunk: u32,
    pub sample_count: u32,
}

pub fn extract_sample_data(moov_box: MOOVBox) -> Result<VecDeque<SampleData>, Box<dyn Error>> {
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

    chunk_data_vec
        .iter()
        .flat_map(|chunk| {
            chunk
                .sample_sizes
                .iter()
                .map(|size| {
                    let sample_data: SampleData = (*size, chunk.is_video);
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
        let tuple = (
            chunk_offset.clone(),
            chunk_to_sample[current_sample_count_index].sample_count,
        );

        if chunk_to_sample.len() > next_sample_count_index {
            if chunk_index + 1 >= chunk_to_sample[next_sample_count_index].starting_chunk as usize {
                current_sample_count_index = next_sample_count_index;
                next_sample_count_index += 1;
            }
        }

        if chunk_index == 0 {
            println!(
                "First chunk with offset {} has {} samples",
                chunk_offset, tuple.1
            );
        }

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
                sample_count: sample_count.clone(),
                sample_sizes: vec![general_size; sample_count.clone() as usize],
            })
            .collect();
    }

    let sizes: Vec<u32> = data[2..].to_vec();
    let mut current_index = 0;

    chunk_offsets_with_sample_count
        .iter()
        .map(|(offset, sample_count)| {
            let chunk_data = ChunkData {
                is_video,
                offset: offset.clone(),
                sample_count: sample_count.clone(),
                sample_sizes: sizes[current_index..(current_index + *sample_count as usize)]
                    .to_vec(),
            };
            current_index += *sample_count as usize;

            println!("Chunk data: {:?}", chunk_data);

            chunk_data
        })
        .collect()
}
