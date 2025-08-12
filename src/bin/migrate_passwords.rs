use sam::sam::security::Auth;
use sam::sam::memory::{Human, PostgresQueries, PGCol};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting password migration to secure hashes...");
    
    // Get all humans with plaintext passwords
    let humans = Human::select(None, None, None, None)?;
    
    let mut migrated_count = 0;
    let mut error_count = 0;
    
    for mut human in humans {
        if let Some(password) = &human.password {
            // Check if already hashed (argon2 hashes start with $argon2)
            if !password.starts_with("$argon2") {
                println!("Migrating password for user: {}", human.email.as_ref().unwrap_or(&"Unknown".to_string()));
                
                // Hash the plaintext password
                match Auth::hash_password(password) {
                    Ok(hashed) => {
                        human.password = Some(hashed);
                        match human.save() {
                            Ok(_) => {
                                migrated_count += 1;
                                println!("  ✓ Successfully migrated");
                            }
                            Err(e) => {
                                error_count += 1;
                                eprintln!("  ✗ Failed to save: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        error_count += 1;
                        eprintln!("  ✗ Failed to hash password: {}", e);
                    }
                }
            } else {
                println!("User {} already has hashed password, skipping", 
                    human.email.as_ref().unwrap_or(&"Unknown".to_string()));
            }
        }
    }
    
    println!("\nMigration complete!");
    println!("  Migrated: {} passwords", migrated_count);
    if error_count > 0 {
        println!("  Errors: {} passwords failed to migrate", error_count);
    }
    
    Ok(())
}