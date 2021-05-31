use bls_signatures_rs::bn256::Bn256;
use bls_signatures_rs::MultiSignature;
use bounce::bounce_satellite_client::BounceSatelliteClient;
use bounce::{commit::CommitType, configure_log, configure_log_to_file, Commit};
use clap::{crate_authors, crate_version, App, Arg};
use log::info;
use rand::{thread_rng, Rng};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = App::new("Bounce ground station")
        .version(crate_version!())
        .author(crate_authors!())
        .arg(
            Arg::with_name("addr")
                .short("a")
                .value_name("ADDRESS")
                .help("Specify an alternate address to connect to.")
                .default_value("0.0.0.0"),
        )
        .arg(
            Arg::with_name("port")
                .short("p")
                .value_name("PORT")
                .help("Specify an alternate port to connect to.")
                .default_value("50051"),
        )
        .arg(
            Arg::with_name("log_to_stdout")
                .help("By default logs are saved to files, if set log only to stdout.")
                .takes_value(false),
        )
        .get_matches();

    let addr = matches.value_of("addr").unwrap();
    let port = matches.value_of("port").unwrap();
    let log_to_stdout = matches.is_present("log_to_stdout");

    if log_to_stdout {
        configure_log()?;
    } else {
        configure_log_to_file("ground-station")?;
    }

    let dst = format!("http://{}:{}", addr, port);

    let mut client = BounceSatelliteClient::connect(dst).await?;

    let msg = chrono::Utc::now().to_rfc2822();
    info!("Message to send: {}", msg);

    let mut rng = thread_rng();
    let ground_station_private_key: Vec<u8> = (0..32).map(|_| rng.gen()).collect();
    let ground_station_public_key = Bn256
        .derive_public_key(&ground_station_private_key)
        .unwrap();
    let signature = Bn256
        .sign(&ground_station_private_key, &msg.as_bytes())
        .unwrap();

    let precommit = Commit {
        typ: CommitType::Precommit.into(),
        i: 1,
        j: 0,
        msg: msg.as_bytes().to_vec(),
        public_key: ground_station_public_key,
        signature,
        aggregated: false,
    };

    let request = tonic::Request::new(precommit);

    let response = client.bounce(request).await?.into_inner();

    let _ = Bn256
        .verify(&response.signature, &msg.as_bytes(), &response.public_key)
        .unwrap();

    info!("Verified the message was signed by the cubesat.");
    Ok(())
}
