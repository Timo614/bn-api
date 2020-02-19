use bigneon_db::models::*;
use functional::base;

#[cfg(test)]
mod index_tests {
    use super::*;
    #[test]
    fn index_org_member() {
        base::chat_workflows::index(Roles::OrgMember, false);
    }
    #[test]
    fn index_admin() {
        base::chat_workflows::index(Roles::Admin, true);
    }
    #[test]
    fn index_super() {
        base::chat_workflows::index(Roles::Super, true);
    }
    #[test]
    fn index_user() {
        base::chat_workflows::index(Roles::User, false);
    }
    #[test]
    fn index_org_owner() {
        base::chat_workflows::index(Roles::OrgOwner, false);
    }
    #[test]
    fn index_door_person() {
        base::chat_workflows::index(Roles::DoorPerson, false);
    }
    #[test]
    fn index_promoter() {
        base::chat_workflows::index(Roles::Promoter, false);
    }
    #[test]
    fn index_promoter_read_only() {
        base::chat_workflows::index(Roles::PromoterReadOnly, false);
    }
    #[test]
    fn index_org_admin() {
        base::chat_workflows::index(Roles::OrgAdmin, false);
    }
    #[test]
    fn index_box_office() {
        base::chat_workflows::index(Roles::OrgBoxOffice, false);
    }
}

#[cfg(test)]
mod show_tests {
    use super::*;
    #[test]
    fn show_org_member() {
        base::chat_workflows::show(Roles::OrgMember, false);
    }
    #[test]
    fn show_admin() {
        base::chat_workflows::show(Roles::Admin, true);
    }
    #[test]
    fn show_super() {
        base::chat_workflows::show(Roles::Super, true);
    }
    #[test]
    fn show_user() {
        base::chat_workflows::show(Roles::User, false);
    }
    #[test]
    fn show_org_owner() {
        base::chat_workflows::show(Roles::OrgOwner, false);
    }
    #[test]
    fn show_door_person() {
        base::chat_workflows::show(Roles::DoorPerson, false);
    }
    #[test]
    fn show_promoter() {
        base::chat_workflows::show(Roles::Promoter, false);
    }
    #[test]
    fn show_promoter_read_only() {
        base::chat_workflows::show(Roles::PromoterReadOnly, false);
    }
    #[test]
    fn show_org_admin() {
        base::chat_workflows::show(Roles::OrgAdmin, false);
    }
    #[test]
    fn show_box_office() {
        base::chat_workflows::show(Roles::OrgBoxOffice, false);
    }
}

#[cfg(test)]
mod create_tests {
    use super::*;
    #[test]
    fn create_org_member() {
        base::chat_workflows::create(Roles::OrgMember, false);
    }
    #[test]
    fn create_admin() {
        base::chat_workflows::create(Roles::Admin, true);
    }
    #[test]
    fn create_super() {
        base::chat_workflows::create(Roles::Super, true);
    }
    #[test]
    fn create_user() {
        base::chat_workflows::create(Roles::User, false);
    }
    #[test]
    fn create_org_owner() {
        base::chat_workflows::create(Roles::OrgOwner, false);
    }
    #[test]
    fn create_door_person() {
        base::chat_workflows::create(Roles::DoorPerson, false);
    }
    #[test]
    fn create_promoter() {
        base::chat_workflows::create(Roles::Promoter, false);
    }
    #[test]
    fn create_promoter_read_only() {
        base::chat_workflows::create(Roles::PromoterReadOnly, false);
    }
    #[test]
    fn create_org_admin() {
        base::chat_workflows::create(Roles::OrgAdmin, false);
    }
    #[test]
    fn create_box_office() {
        base::chat_workflows::create(Roles::OrgBoxOffice, false);
    }
}

#[cfg(test)]
mod update_tests {
    use super::*;
    #[test]
    fn update_org_member() {
        base::chat_workflows::update(Roles::OrgMember, false);
    }
    #[test]
    fn update_admin() {
        base::chat_workflows::update(Roles::Admin, true);
    }
    #[test]
    fn update_super() {
        base::chat_workflows::update(Roles::Super, true);
    }
    #[test]
    fn update_user() {
        base::chat_workflows::update(Roles::User, false);
    }
    #[test]
    fn update_org_owner() {
        base::chat_workflows::update(Roles::OrgOwner, false);
    }
    #[test]
    fn update_door_person() {
        base::chat_workflows::update(Roles::DoorPerson, false);
    }
    #[test]
    fn update_promoter() {
        base::chat_workflows::update(Roles::Promoter, false);
    }
    #[test]
    fn update_promoter_read_only() {
        base::chat_workflows::update(Roles::PromoterReadOnly, false);
    }
    #[test]
    fn update_org_admin() {
        base::chat_workflows::update(Roles::OrgAdmin, false);
    }
    #[test]
    fn update_box_office() {
        base::chat_workflows::update(Roles::OrgBoxOffice, false);
    }
}

#[cfg(test)]
mod publish_tests {
    use super::*;
    #[test]
    fn publish_org_member() {
        base::chat_workflows::publish(Roles::OrgMember, false);
    }
    #[test]
    fn publish_admin() {
        base::chat_workflows::publish(Roles::Admin, true);
    }
    #[test]
    fn publish_super() {
        base::chat_workflows::publish(Roles::Super, true);
    }
    #[test]
    fn publish_user() {
        base::chat_workflows::publish(Roles::User, false);
    }
    #[test]
    fn publish_org_owner() {
        base::chat_workflows::publish(Roles::OrgOwner, false);
    }
    #[test]
    fn publish_door_person() {
        base::chat_workflows::publish(Roles::DoorPerson, false);
    }
    #[test]
    fn publish_promoter() {
        base::chat_workflows::publish(Roles::Promoter, false);
    }
    #[test]
    fn publish_promoter_read_only() {
        base::chat_workflows::publish(Roles::PromoterReadOnly, false);
    }
    #[test]
    fn publish_org_admin() {
        base::chat_workflows::publish(Roles::OrgAdmin, false);
    }
    #[test]
    fn publish_box_office() {
        base::chat_workflows::publish(Roles::OrgBoxOffice, false);
    }
}

#[cfg(test)]
mod destroy_tests {
    use super::*;
    #[test]
    fn destroy_org_member() {
        base::chat_workflows::destroy(Roles::OrgMember, false);
    }
    #[test]
    fn destroy_admin() {
        base::chat_workflows::destroy(Roles::Admin, true);
    }
    #[test]
    fn destroy_super() {
        base::chat_workflows::destroy(Roles::Super, true);
    }
    #[test]
    fn destroy_user() {
        base::chat_workflows::destroy(Roles::User, false);
    }
    #[test]
    fn destroy_org_owner() {
        base::chat_workflows::destroy(Roles::OrgOwner, false);
    }
    #[test]
    fn destroy_door_person() {
        base::chat_workflows::destroy(Roles::DoorPerson, false);
    }
    #[test]
    fn destroy_promoter() {
        base::chat_workflows::destroy(Roles::Promoter, false);
    }
    #[test]
    fn destroy_promoter_read_only() {
        base::chat_workflows::destroy(Roles::PromoterReadOnly, false);
    }
    #[test]
    fn destroy_org_admin() {
        base::chat_workflows::destroy(Roles::OrgAdmin, false);
    }
    #[test]
    fn destroy_box_office() {
        base::chat_workflows::destroy(Roles::OrgBoxOffice, false);
    }
}
