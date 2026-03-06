#![cfg(feature = "functions")]
use subset::Subset;

#[allow(dead_code)]
struct UserMetadata {
    followers: usize,
    following: usize,
    last_login: chrono::DateTime<chrono::Utc>,
}

#[allow(dead_code)]
struct User {
    username: String,
    email: String,
    metadata: UserMetadata,
}

impl User {
    fn calculate_follower_ratio(&self) -> f64 {
        self.metadata.followers as f64 / self.metadata.following as f64
    }
}

#[derive(Subset)]
#[subset(from = "User")]
#[subset(functions = ["from::calculate_follower_ratio"])]
struct PublicUser {
    username: String,
    #[subset(path = "metadata.followers")]
    followers: usize,
    #[subset(path = "metadata.following")]
    following: usize,
}

#[derive(Subset)]
#[subset(from = "PublicUser")]
#[subset(functions = ["from::calculate_follower_ratio"])]
struct AliasedPublicUser {
    username: String,
    #[subset(alias = "followers")]
    aliased_followers: usize,
    #[subset(alias = "following")]
    aliased_following: usize,
}

#[test]
fn functions_construct_derivatively_through_path_and_alias() {
    let user = User {
        username: "Jerry".to_string(),
        email: String::new(),
        metadata: UserMetadata {
            following: 1,
            followers: 2,
            last_login: chrono::DateTime::<chrono::Utc>::MIN_UTC,
        },
    };
    assert_eq!(2.0, user.calculate_follower_ratio());
    let public_user: PublicUser = user.into();
    assert_eq!(public_user.username, "Jerry");
    assert_eq!(public_user.followers, 2);
    assert_eq!(2.0, public_user.calculate_follower_ratio());
    let aliased_public_user: AliasedPublicUser = public_user.into();
    assert_eq!(2.0, aliased_public_user.calculate_follower_ratio());
}
