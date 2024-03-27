
use glass_mu1603::*;
use rusb::{ Context, UsbContext, Device, DeviceHandle, DeviceDescriptor };
use std::io::Write;

fn main() {

    let mut ctx = Context::new()
        .expect("[!] Couldn't create usb context");

    let mut cam = Mu1603::try_open(&mut ctx)
        .expect("[!] Couldn't open camera");

    cam.start_stream(Mu1603Mode::Mode1)
        .expect("[!] Couldn't start stream");

    let mut frames = Vec::new();
    while frames.len() < 5 {
        match cam.try_read_frame() {
            Ok(data) => {
                println!("[*] Got frame");
                frames.push(data);
            },
            Err(Mu1603Error::FirstFrame) => {
                println!("[*] Skipped first frame");
                continue;
            },
            Err(Mu1603Error::NotStreaming) => {
                println!("[*] Not streaming?");
                break;
            },
            Err(e) => {
                println!("Error: {:?}", e);
                break;
            },
        }
    }
    cam.stop_stream().unwrap();


    for (idx, frame) in frames.iter().enumerate() {
        let name = format!("{:04}.rggb8.raw", idx);
        let path = format!("/tmp/{}", name);
        let mut f = std::fs::File::create(&path).unwrap();
        f.write(frame).unwrap();
        println!("[*] Wrote frame to {}", &path);

    }

}
