use std::error::Error;

#[derive(Debug)]
pub enum Streams {
    Audio,
    Video,
}

pub struct MP4Box {
    pub size: u32,
    pub title: String,
    pub data: Vec<u8>,
}

pub struct FTYPBox {
    pub size: u32,
    pub data: Vec<u8>,
}

pub struct MVHDBox {
    pub size: u32,
    pub data: Vec<u8>,
}

pub struct STSDBox {
    pub size: u32,
    pub data: Vec<u8>,
}

pub struct STTSBox {
    pub size: u32,
    pub data: Vec<u8>,
}

pub struct CTTSBox {
    pub size: u32,
    pub data: Vec<u8>,
}

pub struct STSCBox {
    pub size: u32,
    pub data: Vec<u8>,
}

pub struct STSZBox {
    pub size: u32,
    pub data: Vec<u8>,
}

pub struct STCOBox {
    pub size: u32,
    pub data: Vec<u8>,
}

pub struct STSSBox {
    pub size: u32,
    pub data: Vec<u8>,
}

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

pub struct DINFBox {
    pub size: u32,
    pub data: Vec<u8>,
}

pub struct VMHDBox {
    pub size: u32,
    pub data: Vec<u8>,
}

pub struct SMHDBox {
    pub size: u32,
    pub data: Vec<u8>,
}

pub struct MINFBox {
    pub size: u32,
    pub header: Streams,
    pub dinf: DINFBox,
    pub stbl: STBLBbox,
}

pub struct HDLRBox {
    pub size: u32,
    data: Vec<u8>,
}

pub struct MDHDBox {
    pub size: u32,
    data: Vec<u8>,
}

pub struct MDIABox {
    pub size: u32,
    pub mdhd: MDHDBox,
    pub hdlr: HDLRBox,
    pub minf: MINFBox,
}

pub struct TKHDBox {
    pub size: u32,
    pub data: Vec<u8>,
}

pub struct TRAKBox {
    pub size: u32,
    pub tkhd: TKHDBox,
    pub media: MDIABox,
}

pub struct MOOVBox {
    pub size: u32,
    pub mvhd: MVHDBox,
    pub traks: Vec<TRAKBox>,
}

pub enum TopBoxes {
    Moov(MOOVBox),
    Ftyp(FTYPBox),
}

pub fn parse_moov(size: u32, mut data: Vec<u8>) -> Result<MOOVBox, Box<dyn Error>> {
    let mut mvhd_box = None;
    let mut traks: Vec<TRAKBox> = vec![];

    while data.len() > 0 {
        let size_bytes: [u8; 4] = data.drain(..4).collect::<Vec<u8>>().try_into().unwrap();
        let box_size = u32::from_be_bytes(size_bytes);

        let title_bytes: [u8; 4] = data.drain(..4).collect::<Vec<u8>>().try_into().unwrap();
        let title = String::from_utf8_lossy(&title_bytes);

        match title.to_string().as_str() {
            "mvhd" => {
                mvhd_box = Some(MVHDBox {
                    size: box_size,
                    data: data.drain(..(box_size - 8) as usize).collect(),
                });
            }
            "trak" => {
                let trak_box =
                    parse_trak(box_size, data.drain(..(box_size - 8) as usize).collect())?;

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

pub fn parse_trak(size: u32, mut data: Vec<u8>) -> Result<TRAKBox, Box<dyn Error>> {
    let mut tkhd_box = None;
    let mut mdia_box = None;

    while data.len() > 0 {
        let size_bytes: [u8; 4] = data.drain(..4).collect::<Vec<u8>>().try_into().unwrap();
        let box_size = u32::from_be_bytes(size_bytes);

        let title_bytes: [u8; 4] = data.drain(..4).collect::<Vec<u8>>().try_into().unwrap();
        let title = String::from_utf8_lossy(&title_bytes);

        match title.to_string().as_str() {
            "tkhd" => {
                tkhd_box = Some(TKHDBox {
                    size: box_size,
                    data: data.drain(..(box_size - 8) as usize).collect(),
                });
            }
            "mdia" => {
                mdia_box = Some(parse_mdia(
                    box_size,
                    data.drain(..(box_size - 8) as usize).collect(),
                )?);
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

pub fn parse_mdia(size: u32, mut data: Vec<u8>) -> Result<MDIABox, Box<dyn Error>> {
    let mut mdhd_box = None;
    let mut hdlr_box = None;
    let mut minf_box = None;

    while data.len() > 0 {
        let size_bytes: [u8; 4] = data.drain(..4).collect::<Vec<u8>>().try_into().unwrap();
        let box_size = u32::from_be_bytes(size_bytes);

        let title_bytes: [u8; 4] = data.drain(..4).collect::<Vec<u8>>().try_into().unwrap();
        let title = String::from_utf8_lossy(&title_bytes);

        match title.to_string().as_str() {
            "mdhd" => {
                mdhd_box = Some(MDHDBox {
                    size: box_size,
                    data: data.drain(..(box_size - 8) as usize).collect(),
                });
            }
            "hdlr" => {
                hdlr_box = Some(HDLRBox {
                    size: box_size,
                    data: data.drain(..(box_size - 8) as usize).collect(),
                });
            }
            "minf" => {
                minf_box = Some(parse_minf(
                    box_size,
                    data.drain(..(box_size - 8) as usize).collect(),
                )?);
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

pub fn parse_minf(size: u32, mut data: Vec<u8>) -> Result<MINFBox, Box<dyn Error>> {
    let mut dinf_box = None;
    let mut stbl_box = None;
    let mut stream_header = None;

    while data.len() > 0 {
        let size_bytes: [u8; 4] = data.drain(..4).collect::<Vec<u8>>().try_into().unwrap();
        let box_size = u32::from_be_bytes(size_bytes);

        let title_bytes: [u8; 4] = data.drain(..4).collect::<Vec<u8>>().try_into().unwrap();
        let title = String::from_utf8_lossy(&title_bytes);

        match title.to_string().as_str() {
            "dinf" => {
                dinf_box = Some(DINFBox {
                    size: box_size,
                    data: data.drain(..(box_size - 8) as usize).collect(),
                });
            }
            "stbl" => {
                stbl_box = Some(parse_stbl(
                    box_size,
                    data.drain(..(box_size - 8) as usize).collect(),
                )?);
            }
            "vmhd" => {
                let _vmhd_box = VMHDBox {
                    size: box_size,
                    data: data.drain(..(box_size - 8) as usize).collect(),
                };
                stream_header = Some(Streams::Video);
            }
            "smhd" => {
                let _smhd_box = SMHDBox {
                    size: box_size,
                    data: data.drain(..(box_size - 8) as usize).collect(),
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

pub fn parse_stbl(size: u32, mut data: Vec<u8>) -> Result<STBLBbox, Box<dyn Error>> {
    let stsd_size_bytes: [u8; 4] = data.drain(..4).collect::<Vec<u8>>().try_into().unwrap();
    let stsd_size = u32::from_be_bytes(stsd_size_bytes);

    let stsd_size_title: [u8; 4] = data.drain(..4).collect::<Vec<u8>>().try_into().unwrap();
    let stsd_title = String::from_utf8_lossy(&stsd_size_title);

    if stsd_title != "stsd" {
        return Err(format!("not stsd, got {}", stsd_title).into());
    }

    let stsd_box = STSDBox {
        size: stsd_size,
        data: data.drain(..(stsd_size - 8) as usize).collect(),
    };

    let mut stts_box: Option<STTSBox> = None;
    let mut ctts_box: Option<CTTSBox> = None;
    let mut stsc_box: Option<STSCBox> = None;
    let mut stsz_box: Option<STSZBox> = None;
    let mut stco_box: Option<STCOBox> = None;
    let mut stss_box: Option<STSSBox> = None;

    while data.len() > 0 {
        let box_size_bytes: [u8; 4] = data.drain(..4).collect::<Vec<u8>>().try_into().unwrap();
        let box_size = u32::from_be_bytes(box_size_bytes);

        let box_title_bytes: [u8; 4] = data.drain(..4).collect::<Vec<u8>>().try_into().unwrap();
        let box_title = String::from_utf8_lossy(&box_title_bytes);

        match box_title.as_ref() {
            "stts" => {
                stts_box = Some(STTSBox {
                    size: box_size,
                    data: data.drain(..(box_size - 8) as usize).collect(),
                });
            }
            "ctts" => {
                ctts_box = Some(CTTSBox {
                    size: box_size,
                    data: data.drain(..(box_size - 8) as usize).collect(),
                });
            }
            "stsc" => {
                stsc_box = Some(STSCBox {
                    size: box_size,
                    data: data.drain(..(box_size - 8) as usize).collect(),
                });
            }
            "stsz" => {
                stsz_box = Some(STSZBox {
                    size: box_size,
                    data: data.drain(..(box_size - 8) as usize).collect(),
                });
            }
            "stco" => {
                stco_box = Some(STCOBox {
                    size: box_size,
                    data: data.drain(..(box_size - 8) as usize).collect(),
                });
            }
            "stss" => {
                stss_box = Some(STSSBox {
                    size: box_size,
                    data: data.drain(..(box_size - 8) as usize).collect(),
                });
            }
            _ => return Err(format!("Unknown stbl sub-box, got {}", box_title).into()),
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
