use bigneon_db::models::*;
use functional::base;

#[cfg(test)]
mod show_tests {
    use super::*;
    #[test]
    fn show_org_member() {
        base::chat_workflow_items::show(Roles::OrgMember, false);
    }
    #[test]
    fn show_admin() {
        base::chat_workflow_items::show(Roles::Admin, true);
    }
    #[test]
    fn show_super() {
        base::chat_workflow_items::show(Roles::Super, true);
    }
    #[test]
    fn show_user() {
        base::chat_workflow_items::show(Roles::User, false);
    }
    #[test]
    fn show_org_owner() {
        base::chat_workflow_items::show(Roles::OrgOwner, false);
    }
    #[test]
    fn show_door_person() {
        base::chat_workflow_items::show(Roles::DoorPerson, false);
    }
    #[test]
    fn show_promoter() {
        base::chat_workflow_items::show(Roles::Promoter, false);
    }
    #[test]
    fn show_promoter_read_only() {
        base::chat_workflow_items::show(Roles::PromoterReadOnly, false);
    }
    #[test]
    fn show_org_admin() {
        base::chat_workflow_items::show(Roles::OrgAdmin, false);
    }
    #[test]
    fn show_box_office() {
        base::chat_workflow_items::show(Roles::OrgBoxOffice, false);
    }
}

#[cfg(test)]
mod create_tests {
    use super::*;
    #[test]
    fn create_org_member() {
        base::chat_workflow_items::create(Roles::OrgMember, false);
    }
    #[test]
    fn create_admin() {
        base::chat_workflow_items::create(Roles::Admin, true);
    }
    #[test]
    fn create_super() {
        base::chat_workflow_items::create(Roles::Super, true);
    }
    #[test]
    fn create_user() {
        base::chat_workflow_items::create(Roles::User, false);
    }
    #[test]
    fn create_org_owner() {
        base::chat_workflow_items::create(Roles::OrgOwner, false);
    }
    #[test]
    fn create_door_person() {
        base::chat_workflow_items::create(Roles::DoorPerson, false);
    }
    #[test]
    fn create_promoter() {
        base::chat_workflow_items::create(Roles::Promoter, false);
    }
    #[test]
    fn create_promoter_read_only() {
        base::chat_workflow_items::create(Roles::PromoterReadOnly, false);
    }
    #[test]
    fn create_org_admin() {
        base::chat_workflow_items::create(Roles::OrgAdmin, false);
    }
    #[test]
    fn create_box_office() {
        base::chat_workflow_items::create(Roles::OrgBoxOffice, false);
    }
}

#[cfg(test)]
mod update_tests {
    use super::*;
    #[test]
    fn update_org_member() {
        base::chat_workflow_items::update(Roles::OrgMember, false);
    }
    #[test]
    fn update_admin() {
        base::chat_workflow_items::update(Roles::Admin, true);
    }
    #[test]
    fn update_super() {
        base::chat_workflow_items::update(Roles::Super, true);
    }
    #[test]
    fn update_user() {
        base::chat_workflow_items::update(Roles::User, false);
    }
    #[test]
    fn update_org_owner() {
        base::chat_workflow_items::update(Roles::OrgOwner, false);
    }
    #[test]
    fn update_door_person() {
        base::chat_workflow_items::update(Roles::DoorPerson, false);
    }
    #[test]
    fn update_promoter() {
        base::chat_workflow_items::update(Roles::Promoter, false);
    }
    #[test]
    fn update_promoter_read_only() {
        base::chat_workflow_items::update(Roles::PromoterReadOnly, false);
    }
    #[test]
    fn update_org_admin() {
        base::chat_workflow_items::update(Roles::OrgAdmin, false);
    }
    #[test]
    fn update_box_office() {
        base::chat_workflow_items::update(Roles::OrgBoxOffice, false);
    }
}

#[cfg(test)]
mod destroy_tests {
    use super::*;
    #[test]
    fn destroy_org_member() {
        base::chat_workflow_items::destroy(Roles::OrgMember, false);
    }
    #[test]
    fn destroy_admin() {
        base::chat_workflow_items::destroy(Roles::Admin, true);
    }
    #[test]
    fn destroy_super() {
        base::chat_workflow_items::destroy(Roles::Super, true);
    }
    #[test]
    fn destroy_user() {
        base::chat_workflow_items::destroy(Roles::User, false);
    }
    #[test]
    fn destroy_org_owner() {
        base::chat_workflow_items::destroy(Roles::OrgOwner, false);
    }
    #[test]
    fn destroy_door_person() {
        base::chat_workflow_items::destroy(Roles::DoorPerson, false);
    }
    #[test]
    fn destroy_promoter() {
        base::chat_workflow_items::destroy(Roles::Promoter, false);
    }
    #[test]
    fn destroy_promoter_read_only() {
        base::chat_workflow_items::destroy(Roles::PromoterReadOnly, false);
    }
    #[test]
    fn destroy_org_admin() {
        base::chat_workflow_items::destroy(Roles::OrgAdmin, false);
    }
    #[test]
    fn destroy_box_office() {
        base::chat_workflow_items::destroy(Roles::OrgBoxOffice, false);
    }
}
