use anchor_lang::prelude::*;

declare_id!("9B1F56Dx649qbEDRbQAXZtmPXTFrLaYjTXBuCeZWMJ1x");

#[program]
pub mod eventsnap {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, oracle: Pubkey) -> Result<()> {
        msg!("Initializing ProgramData: {:?}", ctx.program_id);
        let program_data = &mut ctx.accounts.program_data;
        program_data.owner = ctx.accounts.owner.key();
        program_data.oracle = oracle;
        program_data.event_count = 0;
        Ok(())
    }

    pub fn create_event(
        ctx: Context<CreateEvent>, 
        uid: String,
        name: String,
        banner: String,
        ) -> Result<()> {
            require!(uid.len() <= 32, EventError::UidTooLong);
            require!(name.len() <= 50, EventError::NameTooLong);
            
            let event = &mut ctx.accounts.event;
            let program_data = &mut ctx.accounts.program_data;
            
            event.uid = uid;
            event.name = name;
            event.banner = banner;
            event.owner = ctx.accounts.authority.key();
            event.attendees = vec![ctx.accounts.authority.key()];
            event.highlight_images = vec![];
            
            program_data.event_count = program_data.event_count.checked_add(1)
                .ok_or(EventError::EventCountOverflow)?;
            
            Ok(())
        }

        pub fn join_event(ctx: Context<JoinEvent>) -> Result<()> {  
        let event = &mut ctx.accounts.event;
        let user_data = &mut ctx.accounts.user_data;
        
        require!(!user_data.is_joined, EventError::AlreadyJoined);
        require!(event.attendees.len() < 10, EventError::MaxAttendeesReached);
        
        event.attendees.push(ctx.accounts.authority.key());
        user_data.is_joined = true;
        user_data.uploader_selfie = String::new();
        user_data.images = vec![];
        
        Ok(())
    }

    pub fn upload_image_with_tag(
    ctx: Context<UploadImageWithTag>,
    url: String,
    tag: String
    ) -> Result<()> {
        let user_data = &mut ctx.accounts.user_data;
        let event = &mut ctx.accounts.event;
        
        require!(user_data.is_joined, EventError::NotJoined);
        require!(url.len() <= 200, EventError::UrlTooLong);
        require!(tag.len() <= 50, EventError::TagTooLong);
        require!(user_data.images.len() < 20, EventError::MaxImagesReached);
        
        let image = UploadedImage {
            url,
            tag,
            uploader: ctx.accounts.authority.key(),
        };
        
        user_data.images.push(image.clone());
        event.highlight_images.push(image.url);
        
        Ok(())
    }

    pub fn delete_image(ctx: Context<DeleteImage>, image_index: u32) -> Result<()> {
        let user_data = &mut ctx.accounts.user_data;
        
        require!(
            (image_index as usize) < user_data.images.len(),
            EventError::InvalidImageIndex
        );
        
        user_data.images.remove(image_index as usize);
        Ok(())
    }

    pub fn delete_event(ctx: Context<DeleteEvent>) -> Result<()> {
        let event = &ctx.accounts.event;
        let program_data = &mut ctx.accounts.program_data;
        
        require!(
            event.owner == ctx.accounts.authority.key(),
            EventError::UnauthorizedDeletion
        );
        
        program_data.event_count = program_data.event_count.checked_sub(1)
            .ok_or(EventError::EventCountUnderflow)?;
            
        // Account will be closed automatically due to the close constraint
        Ok(())
    }
    // Fetch all events
    pub fn get_all_events(ctx: Context<GetAllEvents>) -> Result<Vec<Event>> {
        let event = &ctx.accounts.event;
        Ok(vec![event.clone().into_inner()]) // Extract the inner Event struct
    }

    // Fetch all images by event uploaded by the user
    pub fn get_user_images_by_event(ctx: Context<GetUserImagesByEvent>) -> Result<Vec<UploadedImage>> {
        let user_data = &ctx.accounts.user_data;
        let event = &ctx.accounts.event;

        let user_images: Vec<UploadedImage> = user_data.images
            .iter()
            .filter(|image| event.highlight_images.contains(&image.url))
            .cloned()
            .collect();

        Ok(user_images)
    }
}

#[derive(Accounts)]
pub struct GetAllEvents<'info> {
    #[account(mut)]
    pub program_data: Account<'info, ProgramData>,
    #[account(mut)]
    pub event: Account<'info, Event>,
    #[account(mut)]
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct GetUserImagesByEvent<'info> {
    #[account(mut)]
    pub user_data: Account<'info, UserData>,
    #[account(mut)]
    pub event: Account<'info, Event>,
    #[account(mut)]
    pub authority: Signer<'info>,
}

#[error_code]
pub enum EventError {
    #[msg("Event UID must be 32 characters or less")]
    UidTooLong,
    #[msg("Event name must be 50 characters or less")]
    NameTooLong,
    #[msg("URL must be 200 characters or less")]
    UrlTooLong,
    #[msg("Tag must be 50 characters or less")]
    TagTooLong,
    #[msg("User has already joined this event")]
    AlreadyJoined,
    #[msg("User must join event before uploading images")]
    NotJoined,
    #[msg("Invalid image index")]
    InvalidImageIndex,
    #[msg("Only event owner can delete the event")]
    UnauthorizedDeletion,
    #[msg("Event count overflow")]
    EventCountOverflow,
    #[msg("Event count underflow")]
    EventCountUnderflow,
    #[msg("Maximum number of attendees reached")]
    MaxAttendeesReached,
    #[msg("Maximum number of images reached")]
    MaxImagesReached,
}

#[account]
#[derive(Default)]
pub struct ProgramData {
    pub owner: Pubkey,
    pub oracle: Pubkey,
    pub event_count: u64,
}

#[account]
#[derive(Default)]
pub struct Event {
    pub uid: String,
    pub name: String,
    pub banner: String,
    pub owner: Pubkey,
    pub attendees: Vec<Pubkey>,
    pub highlight_images: Vec<String>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default)]
pub struct UploadedImage {
    pub url: String,
    pub tag: String,
    pub uploader: Pubkey,
}

#[account]
#[derive(Default)]
pub struct UserData {
    pub uploader_selfie: String,
    pub is_joined: bool,
    pub images: Vec<UploadedImage>,
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = owner,
        space = 8 + std::mem::size_of::<ProgramData>()
    )]
    pub program_data: Account<'info, ProgramData>,
    #[account(mut)]
    pub owner: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CreateEvent<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + std::mem::size_of::<Event>() + 
            // Space for dynamic vectors
            4 + (32 * 10) + // attendees: up to 10 Pubkeys
            4 + (200 * 10) // highlight_images: up to 50 URLs
    )]
    pub event: Account<'info, Event>,
    #[account(mut)]
    pub program_data: Account<'info, ProgramData>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct JoinEvent<'info> {
    #[account(mut)]
    pub event: Account<'info, Event>,
    #[account(
        init,
        payer = authority,
        space = 8 + std::mem::size_of::<UserData>() +
            // Space for dynamic vectors
            4 + (250 * 20) // images: up to 20 UploadedImages
    )]
    pub user_data: Account<'info, UserData>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UploadImageWithTag<'info> {
    #[account(mut)]
    pub event: Account<'info, Event>,
    #[account(
        mut,
        constraint = user_data.is_joined @ EventError::NotJoined
    )]
    pub user_data: Account<'info, UserData>,
    #[account(mut)]
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct DeleteImage<'info> {
    #[account(
        mut,
        constraint = user_data.is_joined @ EventError::NotJoined
    )]
    pub user_data: Account<'info, UserData>,
    #[account(mut)]
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct DeleteEvent<'info> {
    #[account(
        mut,
        close = authority,
        constraint = event.owner == authority.key() @ EventError::UnauthorizedDeletion
    )]
    pub event: Account<'info, Event>,
    #[account(mut)]
    pub program_data: Account<'info, ProgramData>,
    #[account(mut)]
    pub authority: Signer<'info>,
}
