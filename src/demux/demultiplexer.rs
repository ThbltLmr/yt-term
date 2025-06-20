use std::io::Read;
use std::process::{Command, Stdio};
use std::usize;

use crate::demux::moov::{parse_moov, FTYPBox};

use crate::demux::sample_data::extract_sample_data;

fn demux() {
    let mut yt_dlp_process = Command::new("yt-dlp")
        .args([
            "-o",
            "-",
            "--no-part",
            "-f",
            "18",
            "https://www.youtube.com/watch?v=kFsXTaoP2A4",
        ])
        .stderr(Stdio::null())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Could not start yt-dlp process");

    let mut yt_dlp_stdout = yt_dlp_process.stdout.take().unwrap();

    let mut buffer = vec![0; 1000000];

    let mut accumulated_data: Vec<u8> = vec![];

    let mut ftyp_box = None;
    let mut moov_box = None;
    let mut sample_data = None;

    loop {
        match yt_dlp_stdout.read(&mut buffer) {
            Ok(0) => break,
            Ok(bytes_read) => {
                accumulated_data.extend_from_slice(&buffer[..bytes_read]);

                while accumulated_data.len() >= 8 {
                    let box_size_bytes: [u8; 4] = [
                        accumulated_data[0],
                        accumulated_data[1],
                        accumulated_data[2],
                        accumulated_data[3],
                    ];

                    let box_size = u32::from_be_bytes(box_size_bytes);

                    let box_title_bytes: [u8; 4] = [
                        accumulated_data[4],
                        accumulated_data[5],
                        accumulated_data[6],
                        accumulated_data[7],
                    ];

                    let box_title = String::from_utf8_lossy(&box_title_bytes);

                    if box_title.to_string().as_str() != "mdat"
                        && accumulated_data.len() < box_size as usize
                    {
                        break;
                    }

                    accumulated_data.drain(..8);

                    match box_title.to_string().as_str() {
                        "ftyp" => {
                            ftyp_box = Some(FTYPBox {
                                size: box_size,
                                data: accumulated_data.drain(..(box_size - 8) as usize).collect(),
                            });
                        }
                        "moov" => {
                            match parse_moov(
                                box_size,
                                accumulated_data.drain(..(box_size - 8) as usize).collect(),
                            ) {
                                Ok(ok_box) => {
                                    moov_box = Some(ok_box);
                                }
                                Err(error) => {
                                    panic!("{}", error);
                                }
                            }

                            println!(
                                "Moov box parsed with {} streams",
                                moov_box.as_ref().unwrap().traks.len(),
                            );

                            sample_data = Some(extract_sample_data(moov_box.unwrap()).unwrap());

                            for sample in sample_data.as_ref().unwrap() {
                                println!("Got sample {:?}", sample);
                            }
                        }
                        "mdat" => {
                            if ftyp_box.is_none() {
                                println!("We are f'ed in the B by ftyp");
                            }
                            if sample_data.is_none() {
                                println!("We are f'ed in the B by moov");
                            }
                            println!("This is where the fun begins");
                        }
                        _ => {
                            println!("So this is new, we got a {} box", box_title.to_string());
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Error reading from yt-dlp: {}", e);
            }
        }
    }
}
