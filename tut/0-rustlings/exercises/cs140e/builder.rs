// FIXME: Make me pass! Diff budget: 30 lines.


struct Builder {
    string: Option<String>,
    number: Option<usize>,
}

<<<<<<< HEAD
impl Builder {
    // fn string(...
    //fn string<S>(&mut self, s: S) -> &Builder where S: Into<String> {
        //self.string = Some(s.into());
        //self
    //}
    fn string<S> (&self, s: S) -> Builder where S: Into<String> {
        Builder{string: Some(s.into()), number: self.number}
    }
    fn number(&self, n: usize) -> Builder {
        let new_string: Option<String> = match &self.string {
            Some(s) => Some(s.to_string()),
            None => None
        };
        Builder{string: new_string, number: Some(n)}
    }

    //fn number(&mut self, n: usize) -> &Builder {
        //self.number = Some(n);
        //self
    //}
    // fn number(...
}

impl ToString for Builder {
    // Implement the trait
    fn to_string(&self) -> String {
        let string = match &self.string {
            Some(s) => s.to_string(),
            None => "".to_string()
        };
        let number = match &self.number {
            Some(n) => n.to_string(),
            None => "".to_string()
        };
        if string != String::from("") && number != String::from("") {
            format!("{} {}", string, number)
        } else if string != String::from("") {
            string
        } else if number != String::from("") {
            number
        } else {
            "".to_string()
        }
    }
}

=======
>>>>>>> skeleton/lab2
// Do not modify this function.
#[test]
fn builder() {
    let empty = Builder::default().to_string();
    assert_eq!(empty, "");

    let just_str = Builder::default().string("hi").to_string();
    assert_eq!(just_str, "hi");

    let just_num = Builder::default().number(254).to_string();
    assert_eq!(just_num, "254");

    let a = Builder::default()
        .string("hello, world!")
        .number(200)
        .to_string();

    assert_eq!(a, "hello, world! 200");

    let b = Builder::default()
        .string("hello, world!")
        .number(200)
        .string("bye now!")
        .to_string();

    assert_eq!(b, "bye now! 200");

    let c = Builder::default()
        .string("heap!".to_owned())
        .to_string();

    assert_eq!(c, "heap!");
}
