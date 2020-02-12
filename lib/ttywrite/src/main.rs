mod parsers;

use serial;
use structopt;
use structopt_derive::StructOpt;
use xmodem::Xmodem;

use std::path::PathBuf;
use std::time::Duration;

use structopt::StructOpt;
use serial::core::{CharSize, BaudRate, StopBits, FlowControl, SerialDevice, SerialPortSettings};

use parsers::{parse_width, parse_stop_bits, parse_flow_control, parse_baud_rate};

#[derive(StructOpt, Debug)]
#[structopt(about = "Write to TTY using the XMODEM protocol by default.")]
struct Opt {
    #[structopt(short = "i", help = "Input file (defaults to stdin if not set)", parse(from_os_str))]
    input: Option<PathBuf>,

    #[structopt(short = "b", long = "baud", parse(try_from_str = "parse_baud_rate"),
                help = "Set baud rate", default_value = "115200")]
    baud_rate: BaudRate,

    #[structopt(short = "t", long = "timeout", parse(try_from_str),
                help = "Set timeout in seconds", default_value = "10")]
    timeout: u64,

    #[structopt(short = "w", long = "width", parse(try_from_str = "parse_width"),
                help = "Set data character width in bits", default_value = "8")]
    char_width: CharSize,

    #[structopt(help = "Path to TTY device", parse(from_os_str))]
    tty_path: PathBuf,

    #[structopt(short = "f", long = "flow-control", parse(try_from_str = "parse_flow_control"),
                help = "Enable flow control ('hardware' or 'software')", default_value = "none")]
    flow_control: FlowControl,

    #[structopt(short = "s", long = "stop-bits", parse(try_from_str = "parse_stop_bits"),
                help = "Set number of stop bits", default_value = "1")]
    stop_bits: StopBits,

    #[structopt(short = "r", long = "raw", help = "Disable XMODEM")]
    raw: bool,
}

fn main() {
    use std::fs::File;
    use std::io::{self, BufReader};
    use std::io::Read;
    use std::io::Write;

    let opt = Opt::from_args();
    let mut port = serial::open(&opt.tty_path).expect("path points to invalid TTY");

    // FIXME: Implement the `ttywrite` utility.
    //let mut buf = String::new();
    let progress_fn = |progress| {
        println!("Progress: {:?}", progress);
    };
    match &opt.input {
        Some(fp) => {
            let mut file = File::open(&fp).expect("Input file fails to open");
            //file.read_to_string(&mut buf).expect("Input file cannot be read");
            if opt.raw {
                //let num_bytes = file.read_to_end(port);
                let num_bytes = io::copy(&mut file, &mut port).expect("Raw write failed");
                println!("Wrote {} bytes to output using raw", num_bytes);
            } else {
                println!("xmodem owo");
                //loop {
                    //let mut port = serial::open(&opt.tty_path).expect("path points to invalid TTY");
                    //let mut file = File::open(&fp).expect("Input file fails to open");
                    let num_bytes = Xmodem::transmit_with_progress(file, port, progress_fn).expect("Xmodem transmisison failed");
                    println!("Wrote {} bytes to output using xmodem", num_bytes);
                    //match Xmodem::transmit_with_progress(&mut file, &mut port, progress_fn) {
                        //Ok(n) => {
                            //println!("Wrote {} bytes to output using xmodem", n);
                            //break;
                        //},
                        //Err(_e) => {
                            //println!("{}", e);
                            //continue;
                        //}
                    //}
                //}
            }
        },
        None => {
            let stdin = io::stdin();
            let mut handle = stdin.lock();
            //handle.read_to_string(&mut buf).expect("Cannot read from stdin");
            if opt.raw {
                let num_bytes = io::copy(&mut handle, &mut port).expect("Raw write failed");
                println!("Wrote {} bytes to output using raw", num_bytes);
            } else {
                let num_bytes = Xmodem::transmit_with_progress(handle, port, progress_fn).expect("Xmodem transmission failed");
                println!("Wrote {} bytes to output using xmodem", num_bytes);
            }
        }
    }
}
