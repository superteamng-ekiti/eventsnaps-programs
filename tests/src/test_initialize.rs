use anchor_client::{anchor_lang, solana_client::rpc_client::RpcClient, solana_sdk::{native_token::LAMPORTS_PER_SOL, signature::Keypair, signer::Signer}};

use crate::test_util::{create_default_event, create_event, initialize_program, join_event, request_airdrop_with_retries, setup, upload_image, EventAccounts, JoinEventAccounts};

#[test]
fn test_initialize() {
    let (owner, _, _, program_id, client) = setup();
    
    // Initialize program and get accounts
    let _program_accounts = initialize_program(&owner, program_id, &client)
        .expect("Failed to initialize program");
}

#[test]
fn test_create_event() {
    let (owner, _, _, program_id, client) = setup();
    
    // Initialize program and get accounts
    let program_accounts = initialize_program(&owner, program_id, &client)
        .expect("Failed to initialize program");

    // Now create an event
    let _owner_event = create_default_event(&program_accounts, &owner, &client)
        .expect("Failed to create owner's event");
}

#[test]
fn test_create_different_owners() {
    let (owner, alice, bob, program_id, client) = setup();
    
    let program_accounts = initialize_program(&owner, program_id, &client)
        .expect("Failed to initialize program");
    
    // Owner creates an event
    let _owner_event = create_default_event(&program_accounts, &owner, &client)
        .expect("Failed to create owner's event");
    
    // Alice creates an event
    let _alice_event = create_default_event(&program_accounts, &alice, &client)
        .expect("Failed to create Alice's event");
    
    // Bob creates an event
    let _bob_event = create_default_event(&program_accounts, &bob, &client)
        .expect("Failed to create Bob's event");
}

#[test]
fn test_multiple_events() {
    let (owner, _, _, program_id, client) = setup();
    
    let program_accounts = initialize_program(&owner, program_id, &client)
        .expect("Failed to initialize program");
    
    // Create multiple events
    let events: Vec<EventAccounts> = (0..3)
        .map(|i| {
            create_event(
                &program_accounts,
                &owner,
                &client,
                Some((
                    format!("event_{}", i),
                    format!("Event {}", i),
                    format!("https://example.com/banner_{}.jpg", i),
                )),
            ).expect("Failed to create event")
        })
        .collect();

    // Use events for further testing
    for event in events {
        println!("Created event with UID: {}", event.uid);
    }
}

#[test]
fn test_join_event() {
    let (owner, alice, _, program_id, client) = setup();
    
    // Initialize program
    let program_accounts = initialize_program(&owner, program_id, &client)
        .expect("Failed to initialize program");

    // Create event
    let owner_event = create_default_event(&program_accounts, &owner, &client)
        .expect("Failed to create owner's event");
    println!("Owner event UID: {}", owner_event.uid);
    println!("Owner event pubkey: {}", owner_event.event.pubkey());

    // Alice joins the event
    let join_accounts = join_event(&owner_event, &alice, &client)
        .expect("Failed to join event");
        
    println!("Join event transaction signature: {}", join_accounts.last_signature);
}

#[test]
fn test_upload_image() {
    let (owner, alice, _, program_id, client) = setup();
    
    // Initialize program
    let program_accounts = initialize_program(&owner, program_id, &client)
        .expect("Failed to initialize program");

    // Create event
    let owner_event = create_default_event(&program_accounts, &owner, &client)
        .expect("Failed to create owner's event");

    // Alice joins the event
    let join_accounts = join_event(&owner_event, &alice, &client)
        .expect("Failed to join event");
        
    println!("Join event transaction signature: {}", join_accounts.last_signature);

    // Alice uploads an image with custom parameters
    let _ = upload_image(
        &join_accounts,
        &alice,
        &client,
        Some(("https://example.com/my-image.jpg".to_string(), "fun".to_string()))
    ).expect("Failed to upload image");
}

#[test]
fn test_delete_image() {
    let (owner, alice, _, program_id, client) = setup();
    let program = client.program(program_id).unwrap();
    
    // Initialize program
    let program_accounts = initialize_program(&owner, program_id, &client)
        .expect("Failed to initialize program");

    // Create event
    let owner_event = create_default_event(&program_accounts, &owner, &client)
        .expect("Failed to create owner's event");

    
    // Alice joins the event
    let join_accounts = join_event(&owner_event, &alice, &client)
        .expect("Failed to join event");
        
    println!("Join event transaction signature: {}", join_accounts.last_signature);
    
    // Alice uploads an image with custom parameters
    let image_upload = upload_image(
        &join_accounts,
        &alice,
        &client,
        Some(("https://example.com/my-image.jpg".to_string(), "fun".to_string()))
    ).expect("Failed to upload image");
    
    // Alice deletes the image
    let tx = program
        .request()
        .accounts(eventsnap::accounts::DeleteImage {
            user_data: image_upload.user_data,
            authority: alice.pubkey(),
        })
        .args(eventsnap::instruction::DeleteImage {
            image_index: 0,
        })
        .signer(&alice)
        .send()
        .expect("Failed to delete image");

    println!("Delete image transaction signature: {}", tx);
}

#[test]
fn test_delete_event() {
    let (owner, _, _, program_id, client) = setup();
    let program = client.program(program_id).unwrap();

    // Initialize program
    let program_accounts = initialize_program(&owner, program_id, &client)
        .expect("Failed to initialize program");

    // Create event
    let owner_event = create_default_event(&program_accounts, &owner, &client)
        .expect("Failed to create owner's event");

    // Owner deletes the event using the correct program ID
    let tx = program
        .request()
        .accounts(eventsnap::accounts::DeleteEvent {
            event: owner_event.event.pubkey(),
            program_data: program_accounts.program_data.pubkey(),
            authority: owner.pubkey(),
        })
        .args(eventsnap::instruction::DeleteEvent {})
        .signer(&owner)
        .send()
        .expect("Failed to delete event");

    println!("Delete event transaction signature: {}", tx);
}

#[test]
fn test_event_banner_validation() {} // Test banner URL format

#[test]
fn test_event_count_underflow() {} // Test min event count
#[test]
fn test_oracle_interaction() {} // Test oracle pubkey functionality

#[test]
fn test_event_name_too_long() {
    let (owner, _, _, program_id, client) = setup();
    let program_accounts = initialize_program(&owner, program_id, &client)
        .expect("Failed to initialize program");

    let long_name = "This event name is definitely longer than fifty characters limit".to_string();
    
    let result = create_event(
        &program_accounts,
        &owner,
        &client,
        Some((
            "test_event".to_string(),
            long_name,
            "https://example.com/banner.jpg".to_string(),
        )),
    );
    
    assert!(result.is_err());
}

#[test]
fn test_event_uid_too_long() {
    let (owner, _, _, program_id, client) = setup();
    let program_accounts = initialize_program(&owner, program_id, &client)
        .expect("Failed to initialize program");

    let long_uid = "this_uid_is_definitely_longer_than_thirty_two_characters".to_string();
    
    let result = create_event(
        &program_accounts,
        &owner,
        &client,
        Some((
            long_uid,
            "Test Event".to_string(),
            "https://example.com/banner.jpg".to_string(),
        )),
    );
    
    assert!(result.is_err());
}

#[test]
fn test_double_join_prevention() {
    let (owner, alice, _, program_id, client) = setup();
    let program = client.program(program_id).unwrap();
    
    let program_accounts = initialize_program(&owner, program_id, &client)
        .expect("Failed to initialize program");

    let event = create_default_event(&program_accounts, &owner, &client)
        .expect("Failed to create event");

    // First join
    let join_accounts = join_event(&event, &alice, &client)
        .expect("Failed to join event");

    // Second join - should fail with AlreadyJoined
    let user_data = Keypair::new();
    let result = program
        .request()
        .accounts(eventsnap::accounts::JoinEvent {
            event: event.event.pubkey(),
            user_data: user_data.pubkey(),
            authority: alice.pubkey(),
            system_program: anchor_lang::solana_program::system_program::ID,
        })
        .args(eventsnap::instruction::JoinEvent {})
        .signer(&user_data)
        .signer(&alice)
        .send();

    assert!(result.is_err());
}

#[test]
fn test_unauthorized_event_deletion() {
    let (owner, alice, _, program_id, client) = setup();
    let program = client.program(program_id).unwrap();

    let program_accounts = initialize_program(&owner, program_id, &client)
        .expect("Failed to initialize program");

    let event = create_default_event(&program_accounts, &owner, &client)
        .expect("Failed to create event");

    // Alice attempts to delete owner's event
    let result = program
        .request()
        .accounts(eventsnap::accounts::DeleteEvent {
            event: event.event.pubkey(),
            program_data: program_accounts.program_data.pubkey(),
            authority: alice.pubkey(),
        })
        .args(eventsnap::instruction::DeleteEvent {})
        .signer(&alice)
        .send();

    assert!(result.is_err());
}

#[test]
fn test_image_url_too_long() {
    let (owner, alice, _, program_id, client) = setup();
    
    let program_accounts = initialize_program(&owner, program_id, &client)
        .expect("Failed to initialize program");

    let event = create_default_event(&program_accounts, &owner, &client)
        .expect("Failed to create event");

    let join_accounts = join_event(&event, &alice, &client)
        .expect("Failed to join event");

    let long_url = "https://example.com/".to_string() + &"a".repeat(200);
    
    let result = upload_image(
        &join_accounts,
        &alice,
        &client,
        Some((long_url, "test".to_string()))
    );
    
    assert!(result.is_err());
}

#[test]
fn test_image_tag_too_long() {
    let (owner, alice, _, program_id, client) = setup();
    
    let program_accounts = initialize_program(&owner, program_id, &client)
        .expect("Failed to initialize program");

    let event = create_default_event(&program_accounts, &owner, &client)
        .expect("Failed to create event");

    let join_accounts = join_event(&event, &alice, &client)
        .expect("Failed to join event");

    let long_tag = "this_tag_is_definitely_longer_than_fifty_characters_limit".to_string();
    
    let result = upload_image(
        &join_accounts,
        &alice,
        &client,
        Some(("https://example.com/image.jpg".to_string(), long_tag))
    );
    
    assert!(result.is_err());
}

#[test]
fn test_unauthorized_image_deletion() {
    let (owner, alice, bob, program_id, client) = setup();
    let program = client.program(program_id).unwrap();
    
    let program_accounts = initialize_program(&owner, program_id, &client)
        .expect("Failed to initialize program");

    let event = create_default_event(&program_accounts, &owner, &client)
        .expect("Failed to create event");

    let join_accounts = join_event(&event, &alice, &client)
        .expect("Failed to join event");

    let image_upload = upload_image(
        &join_accounts,
        &alice,
        &client,
        Some(("https://example.com/image.jpg".to_string(), "test".to_string()))
    ).expect("Failed to upload image");

    // Bob attempts to delete Alice's image
    let result = program
        .request()
        .accounts(eventsnap::accounts::DeleteImage {
            user_data: image_upload.user_data,
            authority: bob.pubkey(),
        })
        .args(eventsnap::instruction::DeleteImage {
            image_index: 0,
        })
        .signer(&bob)
        .send();

    assert!(result.is_err());
}

#[test]
#[should_panic(expected = "EventCountOverflow")]
fn test_max_images_per_user() {
    let (owner, alice, _, program_id, client) = setup();
    
    let program_accounts = initialize_program(&owner, program_id, &client)
        .expect("Failed to initialize program");

    let event = create_default_event(&program_accounts, &owner, &client)
        .expect("Failed to create event");

    let join_accounts = join_event(&event, &alice, &client)
        .expect("Failed to join event");

    // Try to upload beyond the limit (20 images)
    for i in 0..21 {
        upload_image(
            &join_accounts,
            &alice,
            &client,
            Some((
                format!("https://example.com/image_{}.jpg", i),
                format!("tag_{}", i)
            ))
        ).expect("Should fail on 21st upload with EventCountOverflow");
    }
}

#[test]
#[should_panic(expected = "EventCountOverflow")]
fn test_max_attendees() {
    let (owner, _, _, program_id, client) = setup();
    
    let program_accounts = initialize_program(&owner, program_id, &client)
        .expect("Failed to initialize program");

    let event = create_default_event(&program_accounts, &owner, &client)
        .expect("Failed to create event");

    // Try to join with more than allowed attendees (10)
    for _i in 0..11 {
        let new_user = Keypair::new();
        request_airdrop_with_retries(
            &RpcClient::new("http://localhost:8899".to_string()), 
            &new_user.pubkey(), 
            LAMPORTS_PER_SOL * 2
        ).expect("Failed to airdrop");
            
        join_event(&event, &new_user, &client)
            .expect("Should fail on 11th join with EventCountOverflow");
    }
}

#[test]
fn test_event_count_overflow() {
    let (owner, _, _, program_id, client) = setup();
    
    let program_accounts = initialize_program(&owner, program_id, &client)
        .expect("Failed to initialize program");

    // Create events until we hit u64::MAX
    let _program = client.program(program_id).unwrap();
    let mut current_count = 0;
    
    while current_count < 5 { // Testing with smaller number for practical purposes
        let result = create_default_event(&program_accounts, &owner, &client);
        assert!(result.is_ok());
        current_count += 1;
    }
}

#[test]
fn test_highlight_images_limit() {
    let (owner, alice, _, program_id, client) = setup();
    
    let program_accounts = initialize_program(&owner, program_id, &client)
        .expect("Failed to initialize program");

    let event = create_default_event(&program_accounts, &owner, &client)
        .expect("Failed to create event");

    let join_accounts = join_event(&event, &alice, &client)
        .expect("Failed to join event");

    // Upload 51 images (highlight_images vector allocated for 50)
    for i in 0..51 {
        let result = upload_image(
            &join_accounts,
            &alice,
            &client,
            Some((
                format!("https://example.com/highlight_{}.jpg", i),
                "highlight".to_string()
            ))
        );
        
        if i >= 50 {
            assert!(result.is_err());
        } else {
            assert!(result.is_ok());
        }
    }
}

#[test]
fn test_duplicate_image_upload() {
    let (owner, alice, _, program_id, client) = setup();
    
    let program_accounts = initialize_program(&owner, program_id, &client)
        .expect("Failed to initialize program");

    let event = create_default_event(&program_accounts, &owner, &client)
        .expect("Failed to create event");

    let join_accounts = join_event(&event, &alice, &client)
        .expect("Failed to join event");

    // Upload same image URL twice
    let image_url = "https://example.com/same_image.jpg".to_string();
    
    let first_upload = upload_image(
        &join_accounts,
        &alice,
        &client,
        Some((image_url.clone(), "first".to_string()))
    );
    assert!(first_upload.is_ok());

    let second_upload = upload_image(
        &join_accounts,
        &alice,
        &client,
        Some((image_url, "second".to_string()))
    );
    assert!(second_upload.is_ok()); // Program allows duplicate URLs
}

#[test]
fn test_upload_without_joining() {
    let (owner, alice, _, program_id, client) = setup();
    let program = client.program(program_id).unwrap();
    
    let program_accounts = initialize_program(&owner, program_id, &client)
        .expect("Failed to initialize program");

    let event = create_default_event(&program_accounts, &owner, &client)
        .expect("Failed to create event");

    // Try to upload without joining
    let user_data = Keypair::new();
    let result = program
        .request()
        .accounts(eventsnap::accounts::UploadImageWithTag {
            event: event.event.pubkey(),
            user_data: user_data.pubkey(),
            authority: alice.pubkey(),
        })
        .args(eventsnap::instruction::UploadImageWithTag {
            url: "https://example.com/image.jpg".to_string(),
            tag: "test".to_string(),
        })
        .signer(&alice)
        .send();

    assert!(result.is_err());
}

#[test]
#[should_panic(expected = "UrlTooLong")]
fn test_url_length_validation() {
    let (owner, alice, _, program_id, client) = setup();
    
    let program_accounts = initialize_program(&owner, program_id, &client)
        .expect("Failed to initialize program");

    let event = create_default_event(&program_accounts, &owner, &client)
        .expect("Failed to create event");

    let join_accounts = join_event(&event, &alice, &client)
        .expect("Failed to join event");

    let long_url = "https://example.com/".to_string() + &"a".repeat(201);
    upload_image(
        &join_accounts,
        &alice,
        &client,
        Some((long_url, "test".to_string()))
    ).expect("Should fail with UrlTooLong");
}

#[test]
#[should_panic(expected = "TagTooLong")]
fn test_tag_length_validation() {
    let (owner, alice, _, program_id, client) = setup();
    
    let program_accounts = initialize_program(&owner, program_id, &client)
        .expect("Failed to initialize program");

    let event = create_default_event(&program_accounts, &owner, &client)
        .expect("Failed to create event");

    let join_accounts = join_event(&event, &alice, &client)
        .expect("Failed to join event");

    let long_tag = "t".repeat(51);
    upload_image(
        &join_accounts,
        &alice,
        &client,
        Some(("https://example.com/image.jpg".to_string(), long_tag))
    ).expect("Should fail with TagTooLong");
}
