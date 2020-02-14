use core::panic::PanicInfo;
use crate::console::{kprintln, CONSOLE};

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    kprintln!("            (");
    kprintln!("       (      )     )");
    kprintln!("         )   (    (");
    kprintln!("        (          `");
    kprintln!("    .-\"\"^\"\"\"^\"\"^\"\"\"^\"\"-.");
    kprintln!("  (//\\\\//\\\\//\\\\//\\\\//\\\\//)");
    kprintln!("   ~\\^^^^^^^^^^^^^^^^^^/~");
    kprintln!("     `================`");
    kprintln!();
    kprintln!("    The pi is overdone.");
    kprintln!();
    kprintln!("---------- PANIC ----------");
    kprintln!("{}", _info);
    loop {}
}
