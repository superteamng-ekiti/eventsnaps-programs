use std::{process::Command, str::FromStr, sync::Arc};
use anchor_client::{
    anchor_lang, solana_client::rpc_client::RpcClient, solana_sdk::{
        commitment_config::CommitmentConfig, native_token::LAMPORTS_PER_SOL, pubkey::Pubkey, signature::{read_keypair_file, Keypair}, signer::Signer
    }, Client, Cluster
};

pub struct ProgramAccounts {
    pub program_data: Keypair,
    pub oracle: Pubkey,
    pub program_id: Pubkey,
    pub last_signature: String,
}

pub struct EventAccounts {
    pub event: Keypair,
    pub uid: String,
    pub name: String,
    pub banner: String,
    pub last_signature: String,
}

pub struct JoinEventAccounts {
    pub user_data: Keypair,
    pub event: Pubkey,
    pub last_signature: String,
}

pub fn request_airdrop_with_retries(rpc_client: &RpcClient, pubkey: &Pubkey, amount: u64) -> Result<(), String> {
    let max_retries = 5;
    let mut current_try = 0;

    while current_try < max_retries {
        match rpc_client.request_airdrop(pubkey, amount) {
            Ok(sig) => {
                let mut confirmed = false;
                for _ in 0..30 {
                    if let Ok(true) = rpc_client.confirm_transaction(&sig) {
                        confirmed = true;
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(500));
                }
                if confirmed {
                    // Verify the balance actually increased
                    std::thread::sleep(std::time::Duration::from_secs(1));
                    if let Ok(balance) = rpc_client.get_balance(pubkey) {
                        if balance >= amount {
                            println!("Successfully airdropped {} SOL", amount as f64 / LAMPORTS_PER_SOL as f64);
                            return Ok(());
                        }
                    }
                }
            }
            Err(e) => println!("Airdrop failed: {}", e),
        }
        current_try += 1;
        if current_try < max_retries {
            println!("Retrying airdrop... (attempt {}/{})", current_try + 1, max_retries);
            std::thread::sleep(std::time::Duration::from_secs(2));
        }
    }
    Err(format!("Failed to airdrop after {} attempts", max_retries))
}

pub fn ensure_test_validator() -> RpcClient {
    let rpc_url = "http://localhost:8899";
    let rpc_client = RpcClient::new(rpc_url);

    // Try to connect to validator
    if rpc_client.get_version().is_err() {
        println!("No validator detected, attempting to start one...");
        // Kill any existing validator process
        Command::new("pkill").args(["-f", "solana-test-validator"]).output().ok();

        // Start new validator
        Command::new("solana-test-validator")
            .arg("--quiet")
            .spawn()
            .expect("Failed to start validator")
            .wait()
            .expect("Failed to wait for validator");

        // Wait for validator to start
        let mut attempts = 0;
        while attempts < 30 {
            if rpc_client.get_version().is_ok() {
                println!("Validator started successfully");
                std::thread::sleep(std::time::Duration::from_secs(2));
                break;
            }
            std::thread::sleep(std::time::Duration::from_secs(1));
            attempts += 1;
        }
        if attempts >= 30 {
            panic!("Failed to start validator after 30 seconds");
        }
    }
    rpc_client
}

pub fn setup() -> (Keypair, Keypair, Keypair, Pubkey, Client<Arc<Keypair>>) {
    let program_id = "9B1F56Dx649qbEDRbQAXZtmPXTFrLaYjTXBuCeZWMJ1x"; // Your program ID
    let anchor_wallet = std::env::var("ANCHOR_WALLET").unwrap();
    let payer = Arc::new(read_keypair_file(&anchor_wallet).unwrap());

    let client = Client::new_with_options(Cluster::Localnet, payer.clone(), CommitmentConfig::confirmed());
    let program_id = Pubkey::from_str(program_id).unwrap();

    // Create wallets for owner, alice and bob
    let owner = Keypair::new();
    let alice = Keypair::new();
    let bob = Keypair::new();

    // Ensure validator is running and get client
    let rpc_client = ensure_test_validator();

    // Fund accounts with smaller amounts and multiple retries
    let fund_amount = LAMPORTS_PER_SOL * 2;
    for (name, kp) in [("owner", &owner), ("alice", &alice), ("bob", &bob)] {
        if let Err(e) = request_airdrop_with_retries(&rpc_client, &kp.pubkey(), fund_amount) {
            panic!("Failed to fund {}: {}", name, e);
        }
    }

    // Return the vault keypair, wallets, program ID, and client for reuse
    (owner, alice, bob, program_id, client)
}

pub fn initialize_program(
    owner: &Keypair,
    program_id: Pubkey,
    client: &Client<Arc<Keypair>>,
) -> Result<ProgramAccounts, Box<dyn std::error::Error>> {
    let program = client.program(program_id)?;
    let program_data = Keypair::new();
    let oracle = Pubkey::new_unique();
    let system_program = anchor_lang::solana_program::system_program::ID;

    let tx = program
        .request()
        .accounts(eventsnap::accounts::Initialize {
            program_data: program_data.pubkey(),
            owner: owner.pubkey(),
            system_program,
        })
        .args(eventsnap::instruction::Initialize { oracle })
        .signer(&program_data)
        .signer(owner)
        .send()?;

    println!("Program initialization signature: {}", tx);

    Ok(ProgramAccounts {
        program_data,  // Move ownership to the struct
        oracle,
        program_id,
        last_signature: tx.to_string(),
    })
}

pub fn create_event(
    program_accounts: &ProgramAccounts,
    authority: &Keypair,
    client: &Client<Arc<Keypair>>,
    event_params: Option<(String, String, String)>,
) -> Result<EventAccounts, Box<dyn std::error::Error>> {
    let program = client.program(program_accounts.program_id)?;
    let event = Keypair::new();
    let system_program = anchor_lang::solana_program::system_program::ID;

    // Use provided parameters or defaults
    let (uid, name, banner) = event_params.unwrap_or_else(|| (
        format!("event{}", rand::random::<u32>()),
        "Test Event".to_string(),
        "https://example.com/banner.jpg".to_string(),
    ));

    let tx = program
        .request()
        .accounts(eventsnap::accounts::CreateEvent {
            event: event.pubkey(),
            program_data: program_accounts.program_data.pubkey(),
            authority: authority.pubkey(),
            system_program,
        })
        .args(eventsnap::instruction::CreateEvent {
            uid: uid.clone(),
            name: name.clone(),
            banner: banner.clone(),
        })
        .signer(&event)
        .signer(authority)
        .send()?;

    println!("Event creation signature: {}", tx);

    Ok(EventAccounts {
        event,
        uid,
        name,
        banner,
        last_signature: tx.to_string(),
    })
}

// Helper function to create an event with default parameters
pub fn create_default_event(
    program_accounts: &ProgramAccounts,
    authority: &Keypair,
    client: &Client<Arc<Keypair>>,
) -> Result<EventAccounts, Box<dyn std::error::Error>> {
    create_event(program_accounts, authority, client, None)
}

pub fn join_event(
    event_accounts: &EventAccounts,
    authority: &Keypair,
    client: &Client<Arc<Keypair>>,
) -> Result<JoinEventAccounts, Box<dyn std::error::Error>> {
    let program = client.program(Pubkey::from_str("9B1F56Dx649qbEDRbQAXZtmPXTFrLaYjTXBuCeZWMJ1x")?)?;
    let user_data = Keypair::new();
    let system_program = anchor_lang::solana_program::system_program::ID;

    let tx = program
        .request()
        .accounts(eventsnap::accounts::JoinEvent {
            event: event_accounts.event.pubkey(),
            user_data: user_data.pubkey(),
            authority: authority.pubkey(),
            system_program,
        })
        .args(eventsnap::instruction::JoinEvent {})
        .signer(&user_data)
        .signer(authority)
        .send()?;

    println!("Join event transaction signature: {}", tx);

    Ok(JoinEventAccounts {
        user_data,
        event: event_accounts.event.pubkey(),
        last_signature: tx.to_string(),
    })
}

pub struct ImageUploadAccounts {
    pub url: String,
    pub tag: String,
    pub event: Pubkey,
    pub user_data: Pubkey,
    pub last_signature: String,
}

pub fn upload_image(
    join_accounts: &JoinEventAccounts,
    authority: &Keypair,
    client: &Client<Arc<Keypair>>,
    image_params: Option<(String, String)>,
) -> Result<ImageUploadAccounts, Box<dyn std::error::Error>> {
    // Use the program ID instead of event pubkey
    let program = client.program(Pubkey::from_str("9B1F56Dx649qbEDRbQAXZtmPXTFrLaYjTXBuCeZWMJ1x")?)?;
    
    let (url, tag) = image_params.unwrap_or_else(|| (
        "https://example.com/image.jpg".to_string(),
        "default".to_string(),
    ));

    let tx = program
        .request()
        .accounts(eventsnap::accounts::UploadImageWithTag {
            event: join_accounts.event,
            user_data: join_accounts.user_data.pubkey(),
            authority: authority.pubkey(),
        })
        .args(eventsnap::instruction::UploadImageWithTag {
            url: url.clone(),
            tag: tag.clone(),
        })
        .signer(authority)
        .send()?;

    println!("Upload image transaction signature: {}", tx);

    Ok(ImageUploadAccounts {
        url,
        tag,
        event: join_accounts.event,
        user_data: join_accounts.user_data.pubkey(),
        last_signature: tx.to_string(),
    })
}

// Helper function to upload an image with default parameters
pub fn upload_default_image(
    join_accounts: &JoinEventAccounts,
    authority: &Keypair,
    client: &Client<Arc<Keypair>>,
) -> Result<ImageUploadAccounts, Box<dyn std::error::Error>> {
    upload_image(join_accounts, authority, client, None)
}
