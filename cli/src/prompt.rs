use std::io::{stdin, stdout, Write};
use termion::input::TermRead;
use termion::event::Key;


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
    let password_input = stdin.read_passwd(&mut stdout);

    if let Ok(Some(password)) = password_input {
        stdout.write_all(b"\n").unwrap();
        (username, password)
    } else {
        stdout.write_all(b"Error\n").unwrap();
        panic!("Failed to get password");
    }
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

    SignupValues { email, username, password }
}

pub struct Credentials {
    pub username: String,
    pub password: String,
}

pub fn login() -> Credentials {
    print!("Please login to continue: ");
    let (username, password) = username_password();

    Credentials { username, password }
}

pub fn is_email_verified() -> bool {
    let stdout = stdout();
    let mut stdout = stdout.lock();
    let stdin = stdin();
    let stdin = stdin.lock();

    stdout.write_all(b"Have you clicked the link in the verification email? [y/n] ")
        .unwrap();
    stdout.flush().expect("Error");

    let c = stdin.keys().next().unwrap();
    stdout.flush().expect("Error");

    match c.unwrap() {
        Key::Char('y') => true,
        Key::Char('n') => false,
        _ => {println!("Sorry didn't catch that"); false},
    }
}
