#![cfg_attr(feature = "no_std", no_std)]

#![feature(decl_macro)]

use shim::io;
use shim::ioerr;

#[cfg(test)] mod tests;
mod read_ext;
mod progress;

pub use progress::{Progress, ProgressFn};

use read_ext::ReadExt;

const SOH: u8 = 0x01;
const EOT: u8 = 0x04;
const ACK: u8 = 0x06;
const NAK: u8 = 0x15;
const CAN: u8 = 0x18;

/// Implementation of the XMODEM protocol.
pub struct Xmodem<R> {
    packet: u8,
    started: bool,
    inner: R,
    progress: ProgressFn
}

impl Xmodem<()> {
    /// Transmits `data` to the receiver `to` using the XMODEM protocol. If the
    /// length of the total data yielded by `data` is not a multiple of 128
    /// bytes, the data is padded with zeroes and sent to the receiver.
    ///
    /// Returns the number of bytes written to `to`, excluding padding zeroes.
    #[inline]
    pub fn transmit<R, W>(data: R, to: W) -> io::Result<usize>
        where W: io::Read + io::Write, R: io::Read
    {
        Xmodem::transmit_with_progress(data, to, progress::noop)
    }

    /// Transmits `data` to the receiver `to` using the XMODEM protocol. If the
    /// length of the total data yielded by `data` is not a multiple of 128
    /// bytes, the data is padded with zeroes and sent to the receiver.
    ///
    /// The function `f` is used as a callback to indicate progress throughout
    /// the transmission. See the [`Progress`] enum for more information.
    ///
    /// Returns the number of bytes written to `to`, excluding padding zeroes.
    pub fn transmit_with_progress<R, W>(mut data: R, to: W, f: ProgressFn) -> io::Result<usize>
        where W: io::Read + io::Write, R: io::Read
    {
        let mut transmitter = Xmodem::new_with_progress(to, f);
        let mut packet = [0u8; 128];
        let mut written = 0;
        'next_packet: loop {
            let n = data.read_max(&mut packet)?;
            packet[n..].iter_mut().for_each(|b| *b = 0);

            if n == 0 {
                transmitter.write_packet(&[])?;
                return Ok(written);
            }

            for _ in 0..10 {
                match transmitter.write_packet(&packet) {
                    Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                    Err(e) => return Err(e),
                    Ok(_) => {
                        written += n;
                        continue 'next_packet;
                    }
                }
            }

            return ioerr!(BrokenPipe, "bad transmit");
        }
    }

    /// Receives `data` from `from` using the XMODEM protocol and writes it into
    /// `into`. Returns the number of bytes read from `from`, a multiple of 128.
    #[inline]
    pub fn receive<R, W>(from: R, into: W) -> io::Result<usize>
       where R: io::Read + io::Write, W: io::Write
    {
        Xmodem::receive_with_progress(from, into, progress::noop)
    }

    /// Receives `data` from `from` using the XMODEM protocol and writes it into
    /// `into`. Returns the number of bytes read from `from`, a multiple of 128.
    ///
    /// The function `f` is used as a callback to indicate progress throughout
    /// the reception. See the [`Progress`] enum for more information.
    pub fn receive_with_progress<R, W>(from: R, mut into: W, f: ProgressFn) -> io::Result<usize>
       where R: io::Read + io::Write, W: io::Write
    {
        let mut receiver = Xmodem::new_with_progress(from, f);
        let mut packet = [0u8; 128];
        let mut received = 0;
        'next_packet: loop {
            for _ in 0..10 {
                match receiver.read_packet(&mut packet) {
                    Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                    Err(e) => return Err(e),
                    Ok(0) => break 'next_packet,
                    Ok(n) => {
                        received += n;
                        into.write_all(&packet)?;
                        continue 'next_packet;
                    }
                }
            }

            return ioerr!(BrokenPipe, "bad receive");
        }

        Ok(received)
    }
}

fn get_checksum(buf: &[u8]) -> u8 {
    return buf.iter().fold(0, |a, b| a.wrapping_add(*b));
}

impl<T: io::Read + io::Write> Xmodem<T> {
    /// Returns a new `Xmodem` instance with the internal reader/writer set to
    /// `inner`. The returned instance can be used for both receiving
    /// (downloading) and sending (uploading).
    pub fn new(inner: T) -> Self {
        Xmodem { packet: 1, started: false, inner, progress: progress::noop}
    }

    /// Returns a new `Xmodem` instance with the internal reader/writer set to
    /// `inner`. The returned instance can be used for both receiving
    /// (downloading) and sending (uploading). The function `f` is used as a
    /// callback to indicate progress throughout the transfer. See the
    /// [`Progress`] enum for more information.
    pub fn new_with_progress(inner: T, f: ProgressFn) -> Self {
        Xmodem { packet: 1, started: false, inner, progress: f }
    }

    /// Reads a single byte from the inner I/O stream. If `abort_on_can` is
    /// `true`, an error of `ConnectionAborted` is returned if the read byte is
    /// `CAN`.
    ///
    /// # Errors
    ///
    /// Returns an error if reading from the inner stream fails or if
    /// `abort_on_can` is `true` and the read byte is `CAN`.
    fn read_byte(&mut self, abort_on_can: bool) -> io::Result<u8> {
        let mut buf = [0u8; 1];
        self.inner.read_exact(&mut buf)?;

        let byte = buf[0];
        if abort_on_can && byte == CAN {
            return ioerr!(ConnectionAborted, "received CAN");
        }

        Ok(byte)
    }

    /// Writes a single byte to the inner I/O stream.
    ///
    /// # Errors
    ///
    /// Returns an error if writing to the inner stream fails.
    fn write_byte(&mut self, byte: u8) -> io::Result<()> {
        self.inner.write_all(&[byte])
    }

    /// Reads a single byte from the inner I/O stream and compares it to `byte`.
    /// If the bytes match, the byte is returned as an `Ok`. If they differ and
    /// the read byte is not `CAN`, an error of `InvalidData` with the message
    /// `expected` is returned. If they differ and the read byte is `CAN`, an
    /// error of `ConnectionAborted` is returned. In either case, if they bytes
    /// differ, a `CAN` byte is written out to the inner stream.
    ///
    /// # Errors
    ///
    /// Returns an error if reading from the inner stream fails, if the read
    /// byte was not `byte`, if the read byte was `CAN` and `byte` is not `CAN`,
    /// or if writing the `CAN` byte failed on byte mismatch.
    fn expect_byte_or_cancel(&mut self, byte: u8, expected: &'static str) -> io::Result<u8> {
        let read_byte = self.read_byte(false)?;
        if read_byte == byte {
            Ok(read_byte)
        } else {
            self.write_byte(CAN)?; //TODO: is this right?
            if read_byte != CAN {
                ioerr!(InvalidData, expected)
            } else {
                ioerr!(ConnectionAborted, "")
            }
        }
    }

    /// Reads a single byte from the inner I/O stream and compares it to `byte`.
    /// If they differ, an error of `InvalidData` with the message `expected` is
    /// returned. Otherwise the byte is returned. If `byte` is not `CAN` and the
    /// read byte is `CAN`, a `ConnectionAborted` error is returned.
    ///
    /// # Errors
    ///
    /// Returns an error if reading from the inner stream fails, or if the read
    /// byte was not `byte`. If the read byte differed and was `CAN`, an error
    /// of `ConnectionAborted` is returned. Otherwise, the error kind is
    /// `InvalidData`.
    fn expect_byte(&mut self, byte: u8, expected: &'static str) -> io::Result<u8> {
        let mut buf = [0u8; 1];
        self.inner.read_exact(&mut buf)?;
        if buf[0] != byte {
            //Err(io::ErrorKind::InvalidData(expected))
            //io::Error::new(io::ErrorKind::InvalidData, expected)
            ioerr!(InvalidData, expected)
        } else if byte != CAN && buf[0] == CAN {
            //Err(io::ErrorKind::ConnectionAborted)
            //io::Error::new(io::ErrorKind::ConnectionAborted)
            ioerr!(ConnectionAborted, "")
        } else {
            Ok(buf[0])
        }

    }

    /// Reads (downloads) a single packet from the inner stream using the XMODEM
    /// protocol. On success, returns the number of bytes read (always 128).
    ///
    /// The progress callback is called with `Progress::Started` when reception
    /// for the first packet has started and subsequently with
    /// `Progress::Packet` when a packet is received successfully.
    ///
    /// # Errors
    ///
    /// Returns an error if reading or writing to the inner stream fails at any
    /// point. Also returns an error if the XMODEM protocol indicates an error.
    /// In particular, an `InvalidData` error is returned when:
    ///
    ///   * The sender's first byte for a packet isn't `EOT` or `SOH`.
    ///   * The sender doesn't send a second `EOT` after the first.
    ///   * The received packet numbers don't match the expected values.
    ///
    /// An error of kind `Interrupted` is returned if a packet checksum fails.
    ///
    /// An error of kind `ConnectionAborted` is returned if a `CAN` byte is
    /// received when not expected.
    ///
    /// An error of kind `UnexpectedEof` is returned if `buf.len() < 128`.
    pub fn read_packet(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        //TODO: start of transmision stuff
        
        //TODO: assuming here that we are the receiver, but what if we're the sender (sender also
        //receives packets)
        let first_byte = self.read_byte(true)?;
        //let packet_number = self.read_byte(true)?;
        //let packet_number_complement = self.read_byte(true)?;
        if first_byte == EOT {
            //performs end of transmission
            self.write_byte(NAK); //CHECK: does this actually send a byte?
            //'wait: loop { //CHECK: is this the right way to block?
                ////check for second EOT
                //if self.read_byte(false)? == EOT {
                    //break 'wait;
                //}
            //}
            match self.expect_byte(EOT, "second EOT not sent") {
                Ok(n) => {
                    self.write_byte(ACK);
                    Ok(0)
                },
                Err(e) => ioerr!(InvalidData, "second EOT not sent")
            }
        } else if first_byte == SOH {
            self.expect_byte_or_cancel(self.packet + 1, "packet number mismatch");
            self.expect_byte_or_cancel(255 - (self.packet + 1), "packet number 1's complement match error");
            if packet_number != self.packet + 1 || packet_number_complement != 255 - (self.packet + 1) {
                return ioerr!(InvalidData, "packet number doesn't match");
            }
            for i in 0..128 {
                buf[i] = self.read_byte(true)?;
            }
            match self.expect_byte(get_checksum(buf), "checksum fails") {
                Ok(n) => {
                    self.write_byte(ACK);
                    Ok(buf.len())
                },
                Err(e) => {
                    ioerr!(Interrupted, "checksum not equal")
                }
            }
        } else {
            //neither SOH nor EOT
            ioerr!(InvalidData, "first byte is neither SOH nor EOT")
        }
    }

    /// Sends (uploads) a single packet to the inner stream using the XMODEM
    /// protocol. If `buf` is empty, end of transmissions is sent. Users of this
    /// interface should ensure that `write_packet(&[])` is called when data
    /// transmission is complete. On success, returns the number of bytes
    /// written.
    ///
    /// The progress callback is called with `Progress::Waiting` before waiting
    /// for the receiver's `NAK`, `Progress::Started` when transmission of the
    /// first packet has started and subsequently with `Progress::Packet` when a
    /// packet is sent successfully.
    ///
    /// # Errors
    ///
    /// Returns an error if reading or writing to the inner stream fails at any
    /// point. Also returns an error if the XMODEM protocol indicates an error.
    /// In particular, an `InvalidData` error is returned when:
    ///
    ///   * The receiver's first byte isn't a `NAK`.
    ///   * The receiver doesn't respond with a `NAK` to the first `EOT`.
    ///   * The receiver doesn't respond with an `ACK` to the second `EOT`.
    ///   * The receiver responds to a complete packet with something besides
    ///     `ACK` or `NAK`.
    ///
    /// An error of kind `UnexpectedEof` is returned if `buf.len() < 128 &&
    /// buf.len() != 0`.
    ///
    /// An error of kind `ConnectionAborted` is returned if a `CAN` byte is
    /// received when not expected.
    ///
    /// An error of kind `Interrupted` is returned if a packet checksum fails.
    pub fn write_packet(&mut self, buf: &[u8]) -> io::Result<usize> {
        if buf.len() == 0 {
            self.write_byte(EOT);
            (self.progress)(Progress::Waiting);
            match self.expect_byte(NAK, "NAK not sent by receiver") {
                Err(e) => ioerr!(InvalidData, "NAK not sent by the receiver after EOT"),
                Ok(n) => {
                    self.write_byte(EOT);
                    match self.expect_byte(ACK, "ACK not sent by receiver") {
                        Err(e) => ioerr!(InvalidData, "ACK not sent by receiver after second EOT"),
                        Ok(n) => Ok(0)
                    }
                }
            }
        } else {
            if (buf.len() < 128) {
                return ioerr!(UnexpectedEof, "packet length isn't 128 or 0");
            }
            for _ in 0..10 {
                self.write_byte(SOH);
                self.write_byte(self.packet);
                self.write_byte(255 - self.packet);
                self.packet += 1;
                let mut num_bytes = 0;
                for i in 0..128 {
                    if i < buf.len() {
                        self.write_byte(buf[i]);
                        num_bytes += 1;
                    } else {
                        self.write_byte(0);
                    }
                }
                self.write_byte(get_checksum(buf));
                match self.read_byte(true)? {
                    NAK => {
                        continue;
                    },
                    ACK => {
                        return Ok(num_bytes);
                    },
                    _ => {
                        self.write_byte(CAN);
                        return ioerr!(InvalidData, "response to complete packet isn't ACK or NAK");
                    }
                }
            }
            //if it gets here, should have failed 10 times
            self.write_byte(CAN);
            ioerr!(Other, "failed more than 10 times")
        }
    }

    /// Flush this output stream, ensuring that all intermediately buffered
    /// contents reach their destination.
    ///
    /// # Errors
    ///
    /// It is considered an error if not all bytes could be written due to I/O
    /// errors or EOF being reached.
    pub fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}
