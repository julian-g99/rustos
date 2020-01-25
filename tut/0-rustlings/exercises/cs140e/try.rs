// FIXME: Make me compile. Diff budget: 12 line additions and 2 characters.


struct ErrorA;
struct ErrorB;

enum Error {
    A(ErrorA),
    B(ErrorB),
}

impl std::fmt::Debug for ErrorA {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "oh no")
    }
}
impl std::fmt::Debug for ErrorB {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "oh no")
    }
}


// What traits does `Error` need to implement?

fn do_a() -> Result<u16, ErrorA> {
    Err(ErrorA)
}

fn do_b() -> Result<u32, ErrorB> {
    Err(ErrorB)
}

fn do_both() -> Result<(u16, u32), Error> {
    Ok((do_a().unwrap(), do_b().unwrap()))
}

fn main() {}
