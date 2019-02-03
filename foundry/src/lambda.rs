use lambda_runtime::{error::HandlerError, Context};
use serde_derive::{Deserialize, Serialize};


#[derive(Deserialize)]
pub struct CustomEvent {
    #[serde(rename = "firstName")]
    pub first_name: String,
}

#[derive(Serialize)]
pub struct CustomOutput {
    pub message: String,
}

pub fn handler(e: CustomEvent, c: Context) -> Result<CustomOutput, HandlerError> {
    if e.first_name == "" {
        println!("Empty first name in request {}", c.aws_request_id);
        println!("Empty first name");
    }

    Ok(CustomOutput {
        message: format!("Hello, {}!", e.first_name),
    })
}
