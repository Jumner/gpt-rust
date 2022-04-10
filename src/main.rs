use clap::Parser;
use colored::*;
use hyper::body::Buf;
use hyper::{header, Body, Client, Request};
use hyper_tls::HttpsConnector;
use rustyline::Editor;
use serde_derive::{Deserialize, Serialize};
use spinners::*;
use std::env;
use std::error::Error;
use tracing::{debug, Level};

#[derive(Debug, Parser, Serialize)]
#[clap(author, version, about, long_about = None)]
struct GptRequest {
	/// Prompt for GPT
	#[clap(short = 'P', long, default_value = "")]
	prompt: String,
	/// Response Temperature
	#[clap(short, long, default_value_t = 0.3)]
	temperature: f64,
	/// Max tokens to use
	#[clap(short, long, default_value_t = 50)]
	max_tokens: usize,
	/// How Many Responses to generate
	#[clap(short, long, default_value_t = 1)]
	n: u8,
	/// Stop String
	#[clap(short, long, default_value = "")]
	stop: String,
}

#[derive(Debug, Deserialize)]
struct GptChoice {
	text: String,
	index: u8,
	finish_reason: String,
}

#[derive(Debug, Deserialize)]
struct GptResponse {
	id: Option<String>,
	model: Option<String>,
	choices: Option<Vec<GptChoice>>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
	let level = if env::var("DEBUG").is_ok() {
		Level::DEBUG
	} else {
		Level::INFO
	};
	tracing_subscriber::fmt()
		.with_max_level(level)
		// .with_max_level(Level::INFO)
		.pretty()
		.init();

	debug!("Tracing Initialized...");
	debug!("Creating Readine Editor");
	let mut rl = Editor::<()>::new();

	debug!("Parsing args");
	let args = GptRequest::parse();

	debug!("Setting up https connector");
	let https = HttpsConnector::new();

	debug!("Setting up client");
	let client = Client::builder().build(https);
	let uri = "https://api.openai.com/v1/engines/text-davinci-002/completions";

	debug!("Getting Token");
	let token: &str = &env::var("OPENAI_TOKEN").expect("Env var OPENAI_TOKEN not set");
	let header = String::from("Bearer ") + token;

	debug!("Starting Prompt");
	let prompt = rl.readline(&("GPT".cyan().to_string() + &" > ".green().to_string()));
	if prompt.is_err() {
		println!("{}", "Exiting".red());
		return Ok(());
	}
	let prompt = prompt.unwrap();
	let spinner = Spinner::new(Spinners::Material, "Processing".green().to_string());
	let request = GptRequest { prompt, ..args };
	let body = Body::from(serde_json::to_vec(&request)?);
	debug!("Request: {:?}", body);

	debug!("Creating Request");
	let req = Request::post(uri)
		.header(header::CONTENT_TYPE, "application/json")
		.header("Authorization", &header)
		.body(body)
		.expect("Request Failed");

	debug!("Sending Request");
	let res = client.request(req).await?;
	debug!("Got Response, Status: {}", res.status());
	assert!(res.status().is_success());

	debug!("Getting Body");
	let body = hyper::body::aggregate(res).await?;
	spinner.stop();

	debug!("Deserializing Body");
	let json: GptResponse = serde_json::from_reader(body.reader())?;
	debug!("Json Received: {:#?}", json);

	println!(
		"\n\n{} from {}\n{} {}\n",
		"Response Received".green(),
		json
			.model
			.unwrap_or_else(|| String::from("OpenAI"))
			.yellow(),
		"Completion Id: ".cyan(),
		json
			.id
			.unwrap_or_else(|| String::from("Err, Id not found"))
			.yellow()
	);

	for choice in json.choices.expect("No Choices Received") {
		println!(
			"{} {}\n{}\n{} {}\n",
			"Choice".blue(),
			format!("#{}", choice.index + 1).magenta(),
			choice.text,
			"Reason:".yellow(),
			choice.finish_reason.red()
		);
	}

	Ok(())
}
