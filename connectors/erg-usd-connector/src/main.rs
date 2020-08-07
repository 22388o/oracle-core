/// This Connector obtains the nanoErg/USD rate and submits it
/// to an oracle core. It reads the `oracle-config.yaml` to find the port
/// of the oracle core (via Connector-Lib) and submits it to the POST API
/// server on the core.
/// Note: The value that is posted on-chain is the number
/// of nanoErgs per 1 USD, not the rate per nanoErg.
use anyhow::{anyhow, Result};
use connector_lib::{get_core_api_port, OracleCore};
use json;
use std::thread;
use std::time::Duration;

static CONNECTOR_ASCII: &str = r#"
 ______ _____   _____        _    _  _____ _____     _____                            _
|  ____|  __ \ / ____|      | |  | |/ ____|  __ \   / ____|                          | |
| |__  | |__) | |  __ ______| |  | | (___ | |  | | | |     ___  _ __  _ __   ___  ___| |_ ___  _ __
|  __| |  _  /| | |_ |______| |  | |\___ \| |  | | | |    / _ \| '_ \| '_ \ / _ \/ __| __/ _ \| '__|
| |____| | \ \| |__| |      | |__| |____) | |__| | | |___| (_) | | | | | | |  __/ (__| || (_) | |
|______|_|  \_\\_____|       \____/|_____/|_____/   \_____\___/|_| |_|_| |_|\___|\___|\__\___/|_|
==================================================================================================
"#;

static CG_RATE_URL: &str =
    "https://api.coingecko.com/api/v3/simple/price?ids=ergo&vs_currencies=USD";

fn main() {
    // Initialization
    let core_port = get_core_api_port().expect("Failed to read local `oracle-config.yaml`.");
    let oc = OracleCore::new("0.0.0.0", &core_port);

    // Main Loop
    loop {
        // If printing isn't successful (which involves fetching state from core)
        if let Err(e) = print_info(&oc) {
            print!("\x1B[2J\x1B[1;1H");
            println!("{}", CONNECTOR_ASCII);
            println!("Error: {:?}", e);
        }
        // Otherwise if state is accessible
        else {
            let pool_status = oc.pool_status().unwrap();
            let oracle_status = oc.oracle_status().unwrap();

            // Check if Connector should post
            let should_post = &pool_status.current_pool_stage == "Live Epoch"
                && oracle_status.waiting_for_datapoint_submit;

            if should_post {
                let price_res = get_nanoerg_usd_price();
                // If acquiring price worked
                if let Ok(price) = price_res {
                    // If submitting Datapoint tx worked
                    let submit_result = oc.submit_datapoint(price);
                    if let Ok(tx_id) = submit_result {
                        println!("\nSubmit New Datapoint: {} nanoErg/USD", price);
                        println!("Transaction ID: {}", tx_id);
                    } else {
                        println!("Datapoint Tx Submit Error: {:?}", submit_result);
                    }
                } else {
                    println!("{:?}", price_res);
                }
            }
        }

        thread::sleep(Duration::new(30, 0))
    }
}

/// Prints Connector ASCII/info
fn print_info(oc: &OracleCore) -> Result<bool> {
    let pool_status = oc.pool_status()?;
    let oracle_status = oc.oracle_status()?;
    print!("\x1B[2J\x1B[1;1H");
    println!("{}", CONNECTOR_ASCII);
    println!("Current Blockheight: {}", oc.current_block_height()?);
    println!(
        "Current Oracle Pool Stage: {}",
        pool_status.current_pool_stage
    );
    println!(
        "Submit Datapoint In Latest Epoch: {}",
        !oracle_status.waiting_for_datapoint_submit
    );

    println!("Latest Datapoint: {}", oracle_status.latest_datapoint);
    println!("===========================================");
    Ok(true)
}

/// Acquires the nanoErg/USD price from CoinGecko
fn get_nanoerg_usd_price() -> Result<u64> {
    let resp = reqwest::blocking::Client::new().get(CG_RATE_URL).send()?;
    let price_json = json::parse(&resp.text()?)?;
    let price = price_json["ergo"]["usd"].as_f64();
    if let Some(p) = price {
        let nanoerg_price = p * 1000000000.0;
        return Ok(nanoerg_price as u64);
    } else {
        Err(anyhow!("Failed to parse price."))
    }
}
