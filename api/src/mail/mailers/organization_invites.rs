use bigneon_db::models::{Organization, OrganizationInvite};
use config::Config;
use mail::mailers::Mailer;

pub fn invite_user_to_organization_email(
    config: &Config,
    invite: &OrganizationInvite,
    org: &Organization,
    recipient_name: &str,
) -> Mailer {
    let invite_link_accept = format!(
        "{}/invites/accept?token={}",
        config.front_end_url.clone(),
        invite.security_token.expect("Security token is not set")
    );
    let invite_link_decline = format!(
        "{}/invites/decline?token={}",
        config.front_end_url.clone(),
        invite.security_token.expect("Security token is not set")
    );

    Mailer::new(
        config.clone(),
        (invite.user_email.clone(), recipient_name.into()),
        (
            config.mail_from_email.clone(),
            config.mail_from_name.clone(),
        ),
        format!("Invite to join :{}", org.name.clone()),
        format!(
            " Hi {} \r\nThis invite link is valid for 7 days. \r\nIf you want accept the invitation please click this link: {} \r\nIf want to decline please click this link: {}",
            recipient_name,
            invite_link_accept,
            invite_link_decline
        ),
    )
}
