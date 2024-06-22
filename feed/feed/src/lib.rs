use candid::Principal;
use std::cell::RefCell;
use types::{Comment, Like, NewComment, NewLike, NewRepost, Post, Repost, FeedInitArg as InitArg};
use ic_cdk::api::management_canister::main::{CanisterStatusResponse, CanisterIdRecord};

use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{DefaultMemoryImpl, StableBTreeMap, StableCell};

type Memory = VirtualMemory<DefaultMemoryImpl>;

thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> =
        RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));

    static POST_INDEX: RefCell<StableCell<u64, Memory>> = RefCell::new(
        StableCell::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(0))), 
            0
        ).unwrap()
    );

    static POST_MAP: RefCell<StableBTreeMap<u64, Post, Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(1))),
        )
    );

    static FEED_MAP: RefCell<StableBTreeMap<String, Post, Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(2))),
        )
    );

    static BUCKET: RefCell<StableCell<Principal, Memory>> = RefCell::new(
        StableCell::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(3))), 
            Principal::anonymous()
        ).unwrap()
    );

    static ROOT_BUCKET: RefCell<StableCell<Principal, Memory>> = RefCell::new(
        StableCell::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(4))), 
            Principal::anonymous()
        ).unwrap()  
    );
    
    static USER_ACTOR: RefCell<StableCell<Principal, Memory>> = RefCell::new(
        StableCell::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(5))), 
            Principal::anonymous()
        ).unwrap()   
    );
    
    static POST_FETCH_ACTOR: RefCell<StableCell<Principal, Memory>> = RefCell::new(
        StableCell::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(6))), 
            Principal::anonymous()
        ).unwrap()   
    );
    
    static COMMENT_FETCH_ACTOR: RefCell<StableCell<Principal, Memory>> = RefCell::new(
        StableCell::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(7))), 
            Principal::anonymous()
        ).unwrap()   
    );
    
    static LIKE_FETCH_ACTOR: RefCell<StableCell<Principal, Memory>> = RefCell::new(
        StableCell::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(8))), 
            Principal::anonymous()
        ).unwrap()     
    );
    
    static OWNER: RefCell<StableCell<Principal, Memory>> = RefCell::new(
        StableCell::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(9))), 
            Principal::anonymous()
        ).unwrap()      
    );
}

#[ic_cdk::init]
fn init_function(arg: InitArg) {
    ROOT_BUCKET.with(|root_bucket| root_bucket.borrow_mut().set(arg.root_bucket).unwrap());

    USER_ACTOR.with(|user_actor| user_actor.borrow_mut().set(arg.user_actor).unwrap());
    
    POST_FETCH_ACTOR.with(|post_fetch| post_fetch.borrow_mut().set(arg.post_fetch_actor).unwrap());
    
    COMMENT_FETCH_ACTOR.with(|comment_fetch| comment_fetch.borrow_mut().set(arg.comment_fetch_actor).unwrap());
    
    LIKE_FETCH_ACTOR.with(|like_fetch| like_fetch.borrow_mut().set(arg.like_fetch_actor).unwrap());
    
    OWNER.with(|owner| owner.borrow_mut().set(arg.owner).unwrap());
}

// owner
#[ic_cdk::query]
fn get_owner() -> Principal {
    OWNER.with(|pr| pr.borrow().get().clone())
}

fn is_owner() -> Result<(), String>{
    OWNER.with(|owner| {
        assert!(ic_cdk::api::caller() == owner.borrow().get().clone())
    });
    Ok(())
}

#[ic_cdk::update(guard = "is_owner")]
fn update_owner(new_owner: Principal) {
    OWNER.with(|owner| owner.borrow_mut().set(new_owner).unwrap());
}

// Bucket

#[ic_cdk::update]
async fn check_available_bucket() -> bool {
    let call_result = ic_cdk::call::<(), (Option<Principal>, )>(
        ROOT_BUCKET.with(|id| id.borrow().get().clone()), 
        "get_availeable_bucket", 
        ()
    ).await.unwrap().0;
    let availeable_bucket = call_result.unwrap();
    BUCKET.with(|bucket| bucket.borrow_mut().set(availeable_bucket).unwrap());
    true
}

#[ic_cdk::query]
fn get_bucket() -> Option<Principal> {
    BUCKET.with(|pr| {
        if pr.borrow().get().clone() == Principal::anonymous() {
            return None;
        }
        Some(pr.borrow().get().clone())
    })
}

// Post
#[ic_cdk::query]
fn get_post_number() -> u64 {
    POST_MAP.with(|map| map.borrow().len())
}

#[ic_cdk::query]
fn get_post(post_id: String) -> Option<Post> {
    let (bucket, user, index) = check_post_id(&post_id);
    POST_MAP.with(|map| {
        map.borrow().get(&index)
    })
}

#[ic_cdk::query] 
fn get_all_post() -> Vec<Post> {
    POST_MAP.with(|map| {
        let mut post_vec = Vec::new();

        for (k, v) in map.borrow().iter() {
            post_vec.push(v)
        }

        post_vec
    })
}

fn get_post_id(bucket: Principal, user: Principal, index: u64) -> String {
    bucket.to_text() + "#" + &user.to_text() + "#" + &index.to_string()   
}

#[ic_cdk::update(guard = "is_owner")]
async fn create_post(content: String, photo_url: Vec<String>) -> String {
    // get available bucket
    let mut bucket_id = get_bucket();
    if let None = bucket_id {
        check_available_bucket().await;
        bucket_id = get_bucket();
        bucket_id.unwrap();
    };

    // 存储post
    let post = POST_MAP.with(|map| {
        let post_index = POST_INDEX.with(|index| index.borrow().get().clone());
        let post = Post {
            post_id: get_post_id(bucket_id.unwrap(), ic_cdk::caller(), post_index),
            feed_canister: ic_cdk::api::id(),
            index: post_index,
            user: ic_cdk::caller(),
            content: content,
            photo_url: photo_url,
            repost: Vec::new(),
            like: Vec::new(),
            comment: Vec::new(),
            created_at: ic_cdk::api::time()
        };

        map.borrow_mut().insert(post_index, post.clone());
        POST_INDEX.with(|index| index.borrow_mut().set(post_index + 1).unwrap());
        
        post
    });

    // 将帖子内容发送给公共区的 Bucket 
    let call_bucket_result = ic_cdk::call::<(Post, ), (bool, )>(
        bucket_id.unwrap().clone(),
        "store_feed", 
        (post.clone(), )
    ).await.unwrap().0;
    assert!(call_bucket_result);

    // 通知 PostFetch 
      // 查询用户的粉丝
    let followers = ic_cdk::call::<(Principal, ), (Vec<Principal>, )>(
        USER_ACTOR.with(|user| user.borrow().get().clone()), 
        "get_followers_list", 
        (ic_cdk::caller(), )
    ).await.unwrap().0;

      // post_fetch receive
    if followers.len() > 0 {
        let norify_result = ic_cdk::call::<(Vec<Principal>, String, ), ()>(
            POST_FETCH_ACTOR.with(|post_fetch| post_fetch.borrow().get().clone()), 
            "receive_notify", 
            (followers, post.post_id.clone(), )
        ).await.unwrap();
    };

    post.post_id

}

#[ic_cdk::update]
async fn create_repost(post_id: String) -> bool {
    let (bucket, _, post_index) = check_post_id(&post_id);
    let caller = ic_cdk::caller();

    let mut post = POST_MAP.with(|map| map.borrow().get(&post_index)).unwrap();
    
    let mut is_already_repost = false;
    for i in post.repost.iter() {
        if i.user == caller {
            is_already_repost = true;
            break;
        }
    }

    if !is_already_repost {
        post.repost.push(Repost { user: caller, created_at: ic_cdk::api::time()});
        POST_MAP.with(|map| {
            map.borrow_mut().insert(
                post.index, 
                post.clone()
            );
        });

        let new_repost = post.repost;

        // 通知 bucket 更新转发信息
        let call_bucket_result = ic_cdk::call::<(String, NewRepost, ), (bool, )>(
            bucket, 
            "update_post_repost", 
            (post_id.clone(), new_repost, )
        ).await.unwrap().0;
        assert!(call_bucket_result);

        // 获取转发者的粉丝
        let repost_user_followers = ic_cdk::call::<(Principal, ), (Vec<Principal>, )>(
            USER_ACTOR.with(|pr| pr.borrow().get().clone()), 
            "get_followers_list", 
            (ic_cdk::api::caller(), )
        ).await.unwrap().0;

        let mut notify_users: Vec<Principal> = vec![ic_cdk::caller()];

        // 从转发者的粉丝中剔除发帖者本身
        let post_creator = post.user;
        for user in repost_user_followers {
            if user == post_creator {
                continue;
            };
            notify_users.push(user);
        };

        // 通知 PostFetch
        let notify_result = ic_cdk::call::<(Vec<Principal>, String, ), ()>(
            POST_FETCH_ACTOR.with(|post_fetch| post_fetch.borrow().get().clone()), 
            "receive_notify", 
            (notify_users, post_id.clone(), )
        ).await.unwrap();

        true
    } else {
        false
    }
}

#[ic_cdk::update]
async fn create_comment(post_id: String, content: String) -> bool {
    let (bucket, _, index) = check_post_id(&post_id);

    let mut post = POST_MAP.with(|map| map.borrow().get(&index)).unwrap();

    post.comment.push(Comment {
        user: ic_cdk::caller(),
        content: content,
        created_at: ic_cdk::api::time()
    });

    POST_MAP.with(|map| map.borrow_mut().insert(index, post.clone()));

    let new_comment = post.comment;

    // 通知对应的 bucket 更新评论
    let call_bucket_result = ic_cdk::call::<(String, NewComment, ), (bool,)>(
        bucket,
        "update_post_comment",
        (post_id, new_comment, )
    ).await.unwrap().0;

    assert!(call_bucket_result);

    true
}

#[ic_cdk::update]
async fn create_like(post_id: String) -> bool {
    let (bucket, user, index) = check_post_id(&post_id);
    let caller = ic_cdk::caller();

    let mut post = POST_MAP.with(|map| map.borrow().get(&index)).unwrap();

    for i in post.like.iter() {
        if i.user == caller {
            return false;
        }
    }

    post.like.push(Like {
        user: caller,
        created_at: ic_cdk::api::time()
    });

    POST_MAP.with(|map| map.borrow_mut().insert(index, post.clone()));

    let new_like = post.like;

    // 通知 bucket 更新点赞信息
    // bucket中会通知 LikeFetch
    let call_bucket_result = ic_cdk::call::<(String, NewLike, ), (bool, )>(
        bucket, 
        "update_post_like", 
        (post_id, new_like, )
    ).await.unwrap().0;

    assert!(call_bucket_result);

    true
}

// Feed
#[ic_cdk::update]
async fn receive_feed(post_id: String) -> bool {
    if is_feed_in_post(&post_id) {
        return false;
    };
    let (bucket, _, _) = check_post_id(&post_id);
    let call_bucket_result = ic_cdk::call::<(String, ), (Option<Post>, )>(
        bucket, 
        "get_post", 
        (post_id.clone(), )
    ).await.unwrap().0.unwrap();
    FEED_MAP.with(|map| {
        map.borrow_mut().insert(
            post_id, 
            call_bucket_result
        )
    });
    true
}

#[ic_cdk::update]
async fn batch_receive_feed(post_id_array: Vec<String>) {
    for post_id in post_id_array {
        if is_feed_in_post(&post_id) {
            continue;
        }
        let (bucket, _, _) = check_post_id(&post_id);
        let call_bucket_result = ic_cdk::call::<(String, ), (Option<Post>, )>(
            bucket, 
            "get_post", 
            (post_id, )
        ).await.unwrap().0.unwrap();
        FEED_MAP.with(|map| {
            map.borrow_mut().insert(
                call_bucket_result.post_id.clone(), 
                call_bucket_result.clone()
            )
        });
    }
}

#[ic_cdk::update]
async fn receive_comment(post_id: String) -> bool {
    if is_feed_in_post(&post_id) {
        return false;
    }

    let (bucket, _, _) = check_post_id(&post_id);
    let call_bucket_result = ic_cdk::call::<(String, ), (Option<Post>, )>(
        bucket, 
        "get_post", 
        (post_id.clone(), )
    ).await.unwrap().0.unwrap();

    FEED_MAP.with(|map| {
        map.borrow_mut().insert(
            post_id.clone(), 
            call_bucket_result.clone()
        )
    });

    if is_repost_user(call_bucket_result, OWNER.with(|owner| owner.borrow().get().clone())) {
        // 如果该用户是此贴的转发者，则继续向自己的粉丝推流    
        let repost_user_followers = ic_cdk::call::<(Principal, ), (Vec<Principal>, )>(
            USER_ACTOR.with(|user_actor| user_actor.borrow().get().clone()), 
            "get_followers_list",
            (OWNER.with(|owner| owner.borrow().get().clone()), )                                                                                               
        ).await.unwrap().0;

        let call_comment_fetch_result = ic_cdk::call::<(Vec<Principal>, String, ), ()>(
            COMMENT_FETCH_ACTOR.with(|actor| actor.borrow().get().clone()), 
            "receive_repost_user_notify", 
            (repost_user_followers, post_id, )
        ).await.unwrap();
    }

    true
}

#[ic_cdk::update]
async fn batch_receive_comment(post_id_array: Vec<String>) {
    for post_id in post_id_array {
        if is_feed_in_post(&post_id) {
            continue;
        }

        let (bucket, _, _) = check_post_id(&post_id);

        let call_bucket_result = ic_cdk::call::<(String, ), (Option<Post>, )>(
            bucket, 
            "get_post", 
            (post_id.clone(), )
        ).await.unwrap().0.unwrap();

        FEED_MAP.with(|map| {
            map.borrow_mut().insert(
                call_bucket_result.post_id.clone(), 
                call_bucket_result.clone()
            )
        });

        if is_repost_user(call_bucket_result, OWNER.with(|owner| owner.borrow().get().clone())) {
            // 如果该用户是此贴的转发者，则继续向自己的粉丝推流    
            let repost_user_followers = ic_cdk::call::<(Principal, ), (Vec<Principal>, )>(
                USER_ACTOR.with(|user_actor| user_actor.borrow().get().clone()), 
                "get_followers_list",
                (OWNER.with(|owner| owner.borrow().get().clone()), )                                                                                               
            ).await.unwrap().0;
    
            let call_comment_fetch_result = ic_cdk::call::<(Vec<Principal>, String, ), ()>(
                COMMENT_FETCH_ACTOR.with(|actor| actor.borrow().get().clone()), 
                "receive_repost_user_notify", 
                (repost_user_followers, post_id, )
            ).await.unwrap();
        }
    }
}

#[ic_cdk::update]
async fn receive_like(post_id: String) -> bool {
    if is_feed_in_post(&post_id) {
        return false;
    };

    let (bucket, _, _) = check_post_id(&post_id);

    let call_bucket_result = ic_cdk::call::<(String, ), (Option<Post>, )>(
        bucket, 
        "get_post", 
        (post_id.clone(), )
    ).await.unwrap().0.unwrap();

    FEED_MAP.with(|map| {
        map.borrow_mut().insert(
            post_id.clone(), 
            call_bucket_result.clone()
        )
    });

    if is_repost_user(call_bucket_result, OWNER.with(|owner| owner.borrow().get().clone())) {
        // 如果该用户是此贴的转发者，则继续向自己的粉丝推流    
        let repost_user_followers = ic_cdk::call::<(Principal, ), (Vec<Principal>, )>(
            USER_ACTOR.with(|user_actor| user_actor.borrow().get().clone()), 
            "get_followers_list",
            (OWNER.with(|owner| owner.borrow().get().clone()), )                                                                                               
        ).await.unwrap().0;

        let call_like_fetch_result = ic_cdk::call::<(Vec<Principal>, String, ), ()>(
            LIKE_FETCH_ACTOR.with(|actor| actor.borrow().get().clone()), 
            "receive_repost_user_notify", 
            (repost_user_followers, post_id, )
        ).await.unwrap();
    }

    true
}

#[ic_cdk::update]
async fn batch_receive_like(post_id_array: Vec<String>) {
    for post_id in post_id_array {
        if is_feed_in_post(&post_id) {
            continue;
        };
    
        let (bucket, _, _) = check_post_id(&post_id);
    
        let call_bucket_result = ic_cdk::call::<(String, ), (Option<Post>, )>(
            bucket, 
            "get_post", 
            (post_id.clone(), )
        ).await.unwrap().0.unwrap();
        
        FEED_MAP.with(|map| {
            map.borrow_mut().insert(
                call_bucket_result.post_id.clone(), 
                call_bucket_result.clone()
            )
        });
    
        if is_repost_user(call_bucket_result, OWNER.with(|owner| owner.borrow().get().clone())) {
            // 如果该用户是此贴的转发者，则继续向自己的粉丝推流    
            let repost_user_followers = ic_cdk::call::<(Principal, ), (Vec<Principal>, )>(
                USER_ACTOR.with(|user_actor| user_actor.borrow().get().clone()), 
                "get_followers_list",
                (OWNER.with(|owner| owner.borrow().get().clone()), )                                                                                               
            ).await.unwrap().0;
    
            let call_like_fetch_result = ic_cdk::call::<(Vec<Principal>, String, ), ()>(
                LIKE_FETCH_ACTOR.with(|actor| actor.borrow().get().clone()), 
                "receive_repost_user_notify", 
                (repost_user_followers, post_id, )
            ).await.unwrap();
        }
    }
}

#[ic_cdk::query]
fn get_feed_number() -> u64 {
    FEED_MAP.with(|map| {
        map.borrow().len()
    })
}

#[ic_cdk::query]
fn get_feed(post_id: String) -> Option<Post> {
    FEED_MAP.with(|map| {
        map.borrow().get(&post_id)
    })
}

#[ic_cdk::query]
fn get_latest_feed(n: u64) -> Vec<Post> {
    let mut feed_vec: Vec<Post> = FEED_MAP.with(|map| {
        let mut vec: Vec<Post> = Vec::new();

        for (k, v) in map.borrow().iter() {
            vec.push(v)
        }

        vec
    });
    feed_vec.sort_by(|a, b| {
        a.created_at.partial_cmp(&b.created_at).unwrap()
    });

    let mut result: Vec<Post> = Vec::new();
    let mut i = 0;
    for feed in feed_vec.iter().rev() {
        if i >= n {
            break;
        }
        result.push(feed.clone());
        i += 1;
    }

    result
}

#[ic_cdk::update]
async fn status() -> CanisterStatusResponse {
    ic_cdk::api::management_canister::main::canister_status(CanisterIdRecord {
        canister_id: ic_cdk::api::id()
    }).await.unwrap().0
}

fn is_feed_in_post(post_id: &String) -> bool {
    let (bucket, user, index) = check_post_id(post_id);
    POST_MAP.with(|map| {
        if let None = map.borrow().get(&index) {
            return false;
        }
        true
    })
}

fn check_post_id(
    post_id: &String
) -> (Principal, Principal, u64) {
    let words: Vec<&str> = post_id.split("#").collect();
    let bucket = Principal::from_text(words[0]).unwrap();
    let user = Principal::from_text(words[1]).unwrap();
    let post_index = u64::from_str_radix(words[2], 10).unwrap();
    (bucket, user, post_index)
}

fn is_repost_user(post: Post, user: Principal) -> bool {
    for repost in post.repost.iter() {
        if repost.user == user {
            return true;
        }
    }
    false
}

ic_cdk::export_candid!();