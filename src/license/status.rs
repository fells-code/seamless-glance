use crate::license::load::load_license;

pub fn print_license_status() {
    println!("Seamless Glance — License Status");
    println!("--------------------------------");

    match load_license() {
        Ok(license) if license.is_paid() => {
            println!("Type:        Pro Account");
            println!("Expires:     {}", license.expires_at);
        }

        Ok(license) => {
            let days = license.trial_days_remaining().unwrap_or(0);
            println!("Type:        Free Trial");
            println!("Days left:   {}", days);
            println!("Expires:     {}", license.expires_at);
            println!();
            println!("Upgrade at:");
            println!("https://seamlessglance.com");
        }

        Err(err) => {
            println!("Type:        Free Trial");
            println!("Status:      {}", err);
        }
    }
}
