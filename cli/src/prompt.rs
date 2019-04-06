use std::io::{stdin, stdout, Write};
use termion::input::TermRead;


fn username_password() -> (String, String) {
    let stdout = stdout();
    let mut stdout = stdout.lock();
    let stdin = stdin();
    let mut stdin = stdin.lock();

    stdout.write_all(b"Username: ").unwrap();
    stdout.flush().expect("Error");
    let username = stdin.read_line().expect("Fail").unwrap().to_lowercase();

    stdout.write_all(b"Password: ").unwrap();
    stdout.flush().expect("Error");
    let password = stdin.read_passwd(&mut stdout)
        .expect("No password")
        .unwrap();
    // For some reason read_passwd doesn't add a newline
    println!();

    (username, password)
}

fn email() -> String {
    let stdout = stdout();
    let mut stdout = stdout.lock();
    let stdin = stdin();
    let mut stdin = stdin.lock();

    stdout.write_all(b"Email: ").unwrap();
    stdout.flush().expect("Error");
    stdin.read_line().expect("Fail").unwrap().to_lowercase()
}

#[derive(Debug, Clone)]
pub struct SignupValues {
    pub email: String,
    pub username: String,
    pub password: String,
}

pub fn signup() -> SignupValues {
    println!("Please enter the following information");
    let (username, password) = username_password();
    let email = email();

    SignupValues {
        email: email.to_owned(),
        username: username.to_owned(),
        password: password.to_owned()
    }
}

pub struct Credentials {
    pub username: String,
    pub password: String,
}

pub fn login() -> Credentials {
    print!("Please login to continue: ");
    let (username, password) = username_password();

    Credentials {
        username: username.to_owned(),
        password: password.to_owned(),
    }
}
