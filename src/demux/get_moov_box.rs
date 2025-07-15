use std::error::Error;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum Streams {
    Audio,
    Video,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct FTYPBox {
    pub size: u32,
    pub data: Vec<u8>,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct MVHDBox {
    pub size: u32,
    pub data: Vec<u8>,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct STSDBox {
    pub size: u32,
    pub data: Vec<u8>,
    pub avcc: Option<Vec<u8>>,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct STTSBox {
    pub size: u32,
    pub data: Vec<u8>,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct CTTSBox {
    pub size: u32,
    pub data: Vec<u8>,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct STSCBox {
    pub size: u32,
    pub data: Vec<u8>,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct STSZBox {
    pub size: u32,
    pub data: Vec<u8>,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct STCOBox {
    pub size: u32,
    pub data: Vec<u8>,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct STSSBox {
    pub size: u32,
    pub data: Vec<u8>,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct STBLBbox {
    pub size: u32,
    pub stsd: STSDBox,
    pub ctts: Option<CTTSBox>,
    pub stts: STTSBox,
    pub stsz: STSZBox,
    pub stco: STCOBox,
    pub stsc: STSCBox,
    pub stss: Option<STSSBox>,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct DINFBox {
    pub size: u32,
    pub data: Vec<u8>,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct VMHDBox {
    pub size: u32,
    pub data: Vec<u8>,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct SMHDBox {
    pub size: u32,
    pub data: Vec<u8>,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct MINFBox {
    pub size: u32,
    pub header: Streams,
    pub dinf: DINFBox,
    pub stbl: STBLBbox,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct HDLRBox {
    pub size: u32,
    data: Vec<u8>,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct MDHDBox {
    pub size: u32,
    pub data: Vec<u8>,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct MDIABox {
    pub size: u32,
    pub mdhd: MDHDBox,
    pub hdlr: HDLRBox,
    pub minf: MINFBox,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct TKHDBox {
    pub size: u32,
    pub data: Vec<u8>,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct TRAKBox {
    pub size: u32,
    pub tkhd: TKHDBox,
    pub media: MDIABox,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct MOOVBox {
    pub size: u32,
    pub mvhd: MVHDBox,
    pub traks: Vec<TRAKBox>,
}

pub trait DrainToBox {
    fn drain_box_data(&mut self, box_size: u32) -> Vec<u8>;
    fn get_next_box_size_and_title(&mut self) -> (u32, String);
}

impl DrainToBox for Vec<u8> {
    fn drain_box_data(&mut self, box_size: u32) -> Vec<u8> {
        self.drain(..(box_size - 8) as usize).collect()
    }

    fn get_next_box_size_and_title(&mut self) -> (u32, String) {
        let size_bytes: [u8; 4] = self.drain(..4).collect::<Vec<u8>>().try_into().unwrap();
        let box_size = u32::from_be_bytes(size_bytes);

        let title_bytes: [u8; 4] = self.drain(..4).collect::<Vec<u8>>().try_into().unwrap();
        let title = String::from_utf8_lossy(&title_bytes).to_string();

        (box_size, title)
    }
}

pub fn get_moov_box(size: u32, mut data: Vec<u8>) -> Result<MOOVBox, Box<dyn Error>> {
    let mut mvhd_box = None;
    let mut traks: Vec<TRAKBox> = vec![];

    while data.len() > 0 {
        let (box_size, title) = data.get_next_box_size_and_title();

        match title.as_str() {
            "mvhd" => {
                mvhd_box = Some(MVHDBox {
                    size: box_size,
                    data: data.drain_box_data(box_size),
                });
            }
            "trak" => {
                let trak_box = get_trak_box(box_size, data.drain_box_data(box_size))?;

                traks.push(trak_box);
            }
            _ => {
                data.drain(..(box_size - 8) as usize);
            }
        }
    }

    Ok(MOOVBox {
        size,
        mvhd: mvhd_box.ok_or("No mvhd box found")?,
        traks,
    })
}

pub fn get_trak_box(size: u32, mut data: Vec<u8>) -> Result<TRAKBox, Box<dyn Error>> {
    let mut tkhd_box = None;
    let mut mdia_box = None;

    while data.len() > 0 {
        let (box_size, title) = data.get_next_box_size_and_title();

        match title.as_str() {
            "tkhd" => {
                tkhd_box = Some(TKHDBox {
                    size: box_size,
                    data: data.drain_box_data(box_size),
                });
            }
            "mdia" => {
                mdia_box = Some(get_mdia_box(box_size, data.drain_box_data(box_size))?);
            }
            _ => {
                data.drain(..(box_size - 8) as usize);
            }
        }
    }

    Ok(TRAKBox {
        size,
        tkhd: tkhd_box.ok_or("No tkhd box found")?,
        media: mdia_box.ok_or("No mdia box found")?,
    })
}

pub fn get_mdia_box(size: u32, mut data: Vec<u8>) -> Result<MDIABox, Box<dyn Error>> {
    let mut mdhd_box = None;
    let mut hdlr_box = None;
    let mut minf_box = None;

    while data.len() > 0 {
        let (box_size, title) = data.get_next_box_size_and_title();

        match title.as_str() {
            "mdhd" => {
                mdhd_box = Some(MDHDBox {
                    size: box_size,
                    data: data.drain_box_data(box_size),
                });
            }
            "hdlr" => {
                hdlr_box = Some(HDLRBox {
                    size: box_size,
                    data: data.drain_box_data(box_size),
                });
            }
            "minf" => {
                minf_box = Some(get_minf_box(box_size, data.drain_box_data(box_size))?);
            }
            _ => {
                return Err(format!("Unknown mdia sub-box, got {}", title).into());
            }
        }
    }

    Ok(MDIABox {
        size,
        mdhd: mdhd_box.ok_or("No mdhd box found")?,
        hdlr: hdlr_box.ok_or("No hdlr box found")?,
        minf: minf_box.ok_or("No minf box found")?,
    })
}

pub fn get_minf_box(size: u32, mut data: Vec<u8>) -> Result<MINFBox, Box<dyn Error>> {
    let mut dinf_box = None;
    let mut stbl_box = None;
    let mut stream_header = None;

    while data.len() > 0 {
        let (box_size, title) = data.get_next_box_size_and_title();

        match title.as_str() {
            "dinf" => {
                dinf_box = Some(DINFBox {
                    size: box_size,
                    data: data.drain_box_data(box_size),
                });
            }
            "stbl" => {
                stbl_box = Some(get_stbl_box(box_size, data.drain_box_data(box_size))?);
            }
            "vmhd" => {
                let _vmhd_box = VMHDBox {
                    size: box_size,
                    data: data.drain_box_data(box_size),
                };
                stream_header = Some(Streams::Video);
            }
            "smhd" => {
                let _smhd_box = SMHDBox {
                    size: box_size,
                    data: data.drain_box_data(box_size),
                };
                stream_header = Some(Streams::Audio);
            }
            _ => {
                return Err(format!("Unknown minf sub-box, got {}", title).into());
            }
        }
    }

    Ok(MINFBox {
        size,
        header: stream_header.ok_or("No stream header found")?,
        dinf: dinf_box.ok_or("No dinf box found")?,
        stbl: stbl_box.ok_or("No stbl box found")?,
    })
}

pub fn get_stbl_box(size: u32, mut data: Vec<u8>) -> Result<STBLBbox, Box<dyn Error>> {
    let (box_size, title) = data.get_next_box_size_and_title();

    if title != "stsd" {
        return Err(format!("not stsd, got {}", title).into());
    }

    let mut stsd_box = STSDBox {
        size: box_size,
        data: data.drain_box_data(box_size),
        avcc: None,
    };

    let _version_flags = u32::from_be_bytes(stsd_box.data[0..4].try_into().unwrap());
    let entry_count = u32::from_be_bytes(stsd_box.data[4..8].try_into().unwrap());

    if entry_count >= 1 {
        let mut current_offset_in_stsd_data = 8;

        // Read the size and format of the first sample description entry
        if stsd_box.data.len() >= current_offset_in_stsd_data + 8 {
            // Check if enough bytes for size + format
            let _entry_size = u32::from_be_bytes(
                stsd_box.data[current_offset_in_stsd_data..current_offset_in_stsd_data + 4]
                    .try_into()
                    .unwrap(),
            );

            current_offset_in_stsd_data += 4;

            let format_bytes =
                &stsd_box.data[current_offset_in_stsd_data..current_offset_in_stsd_data + 4];
            let format = String::from_utf8_lossy(format_bytes);

            current_offset_in_stsd_data += 4;

            if format == "avc1" {
                let avc_header_fixed_size: usize = 78; // Bytes for avc1 specific fields before any sub-boxes

                let avcc_offset_in_stsd_data = current_offset_in_stsd_data + avc_header_fixed_size;

                if stsd_box.data.len() >= avcc_offset_in_stsd_data + 8 {
                    let avcc_size = u32::from_be_bytes(
                        stsd_box.data[avcc_offset_in_stsd_data..avcc_offset_in_stsd_data + 4]
                            .try_into()
                            .unwrap(),
                    );
                    let avcc_title_bytes =
                        &stsd_box.data[avcc_offset_in_stsd_data + 4..avcc_offset_in_stsd_data + 8];
                    let avcc_title = String::from_utf8_lossy(avcc_title_bytes);

                    if avcc_title == "avcC" {
                        let avcc_data_start = avcc_offset_in_stsd_data + 8;

                        let avcc_data_len = (avcc_size - 8) as usize;
                        let avcc_data_end = avcc_data_start + avcc_data_len;

                        if stsd_box.data.len() >= avcc_data_end {
                            let avcc_data = stsd_box.data[avcc_data_start..avcc_data_end].to_vec();
                            stsd_box.avcc = Some(avcc_data);
                        } else {
                            println!("Not enough data for avcC box content. Expected end: {}, Actual len: {}", avcc_data_end, stsd_box.data.len());
                        }
                    } else {
                        println!("Expected avcC box, but got {}", avcc_title);
                    }
                } else {
                    println!(
                        "Not enough data for avcC box header. Required: {}, Actual: {}",
                        avcc_offset_in_stsd_data + 8,
                        stsd_box.data.len()
                    );
                }
            }
        } else {
            println!("Not enough data for sample description entry size and format.");
        }
    } else {
        println!("No sample description entries found in stsd box.");
    }

    let mut stts_box: Option<STTSBox> = None;
    let mut ctts_box: Option<CTTSBox> = None;
    let mut stsc_box: Option<STSCBox> = None;
    let mut stsz_box: Option<STSZBox> = None;
    let mut stco_box: Option<STCOBox> = None;
    let mut stss_box: Option<STSSBox> = None;

    while data.len() > 0 {
        let (box_size, title) = data.get_next_box_size_and_title();

        match title.as_ref() {
            "stts" => {
                stts_box = Some(STTSBox {
                    size: box_size,
                    data: data.drain_box_data(box_size),
                });
            }
            "ctts" => {
                ctts_box = Some(CTTSBox {
                    size: box_size,
                    data: data.drain_box_data(box_size),
                });
            }
            "stsc" => {
                stsc_box = Some(STSCBox {
                    size: box_size,
                    data: data.drain_box_data(box_size),
                });
            }
            "stsz" => {
                stsz_box = Some(STSZBox {
                    size: box_size,
                    data: data.drain_box_data(box_size),
                });
            }
            "stco" => {
                stco_box = Some(STCOBox {
                    size: box_size,
                    data: data.drain_box_data(box_size),
                });
            }
            "stss" => {
                stss_box = Some(STSSBox {
                    size: box_size,
                    data: data.drain_box_data(box_size),
                });
            }
            _ => return Err(format!("Unknown stbl sub-box, got {}", title).into()),
        }
    }

    Ok(STBLBbox {
        size,
        stsd: stsd_box,
        ctts: ctts_box,
        stts: stts_box.ok_or("stts not found")?,
        stsz: stsz_box.ok_or("stts not found")?,
        stco: stco_box.ok_or("stts not found")?,
        stsc: stsc_box.ok_or("stts not found")?,
        stss: stss_box,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_drain_to_box_trait() {
        let mut data = vec![
            0x00, 0x00, 0x00, 0x10, // size = 16
            0x6D, 0x6F, 0x6F, 0x76, // "moov"
            0x01, 0x02, 0x03, 0x04, // 8 bytes of data (16-8=8)
            0x05, 0x06, 0x07, 0x08,
        ];
        
        let (size, title) = data.get_next_box_size_and_title();
        assert_eq!(size, 16);
        assert_eq!(title, "moov");
        
        let box_data = data.drain_box_data(size);
        assert_eq!(box_data, vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]);
    }

    #[test]
    fn test_get_next_box_size_and_title() {
        let mut data = vec![
            0x00, 0x00, 0x00, 0x18, // size = 24
            0x66, 0x74, 0x79, 0x70, // "ftyp"
            0x69, 0x73, 0x6F, 0x6D, // remaining data
        ];
        
        let (size, title) = data.get_next_box_size_and_title();
        assert_eq!(size, 24);
        assert_eq!(title, "ftyp");
        assert_eq!(data.len(), 4); // Only remaining data left
    }

    #[test]
    fn test_get_moov_box_with_mvhd() {
        let data = vec![
            // mvhd box
            0x00, 0x00, 0x00, 0x10, // size = 16
            0x6D, 0x76, 0x68, 0x64, // "mvhd"
            0x00, 0x00, 0x00, 0x00, // version + flags
            0x01, 0x02, 0x03, 0x04, // timescale (dummy data)
        ];
        
        let result = get_moov_box(24, data);
        assert!(result.is_ok());
        
        let moov_box = result.unwrap();
        assert_eq!(moov_box.size, 24);
        assert_eq!(moov_box.mvhd.size, 16);
        assert_eq!(moov_box.traks.len(), 0);
    }

    #[test]
    fn test_get_moov_box_missing_mvhd() {
        let data = vec![
            // Unknown box instead of mvhd
            0x00, 0x00, 0x00, 0x10, // size = 16
            0x75, 0x6E, 0x6B, 0x6E, // "unkn"
            0x00, 0x00, 0x00, 0x00, // dummy data
            0x01, 0x02, 0x03, 0x04,
        ];
        
        let result = get_moov_box(24, data);
        assert!(result.is_err());
        let error_msg = result.err().unwrap().to_string();
        assert!(error_msg.contains("No mvhd box found"));
    }

    #[test]
    fn test_streams_enum_variants() {
        let video_stream = Streams::Video;
        let audio_stream = Streams::Audio;
        
        // Test that the enum variants exist and can be matched
        match video_stream {
            Streams::Video => assert!(true),
            Streams::Audio => assert!(false),
        }
        
        match audio_stream {
            Streams::Audio => assert!(true),
            Streams::Video => assert!(false),
        }
    }
}
