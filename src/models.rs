use crate::errors::AppError;
use crate::schema::comments;
use crate::schema::posts;
use crate::schema::users;
use diesel::prelude::*;

type Result<T> = std::result::Result<T, AppError>;

#[derive(Queryable, Identifiable, Serialize, Debug, PartialEq)]
pub struct User {
    pub id: i32,
    pub username: String,
}

pub enum UserKey<'a> {
    Username(&'a str),
    ID(i32),
}

#[derive(Queryable, Associations, Identifiable, Serialize, Debug)]
#[belongs_to(User)]
pub struct Post {
    pub id: i32,
    pub user_id: i32,
    pub title: String,
    pub body: String,
    pub published: bool,
}

#[derive(Queryable, Identifiable, Associations, Serialize, Debug)]
#[belongs_to(User)]
#[belongs_to(Post)]
pub struct Comment {
    pub id: i32,
    pub user_id: i32,
    pub post_id: i32,
    pub body: String,
}

#[derive(Queryable, Serialize, Debug)]
pub struct PostWithComment {
    pub id: i32,
    pub title: String,
    pub published: bool,
}

pub fn create_user(conn: &SqliteConnection, username: &str) -> Result<User> {
    conn.transaction(|| {
        diesel::insert_into(users::table)
            .values((users::username.eq(username),))
            .execute(conn)?;

        users::table
            .order(users::id.desc())
            .select((users::id, users::username))
            .first(conn)
            .map_err(Into::into)
    })
}

pub fn find_user<'a>(conn: &SqliteConnection, key: UserKey<'a>) -> Result<User> {
    match key {
        UserKey::Username(name) => users::table
            .filter(users::username.eq(name))
            .select((users::id, users::username))
            .first::<User>(conn)
            .map_err(AppError::from),
        UserKey::ID(id) => users::table
            .find(id)
            .select((users::id, users::username))
            .first::<User>(conn)
            .map_err(Into::into),
    }
}

pub fn create_post(conn: &SqliteConnection, user: &User, title: &str, body: &str) -> Result<Post> {
    conn.transaction(|| {
        diesel::insert_into(posts::table)
            .values((
                posts::user_id.eq(user.id),
                posts::title.eq(title),
                posts::body.eq(body),
            ))
            .execute(conn)?;

        posts::table
            .order(posts::id.desc())
            .select(posts::all_columns)
            .first(conn)
            .map_err(Into::into)
    })
}

pub fn publish_post(conn: &SqliteConnection, post_id: i32) -> Result<Post> {
    conn.transaction(|| {
        diesel::update(posts::table.filter(posts::id.eq(post_id)))
            .set(posts::published.eq(true))
            .execute(conn)?;

        posts::table
            .find(post_id)
            .select(posts::all_columns)
            .first(conn)
            .map_err(Into::into)
    })
}

pub fn all_posts(conn: &SqliteConnection) -> Result<Vec<(Post, User)>> {
    posts::table
        .order(posts::id.desc())
        .filter(posts::published.eq(true))
        .inner_join(users::table)
        .select((posts::all_columns, (users::id, users::username)))
        .load::<(Post, User)>(conn)
        .map_err(Into::into)
}

pub fn user_posts(conn: &SqliteConnection, user_id: i32) -> Result<Vec<Post>> {
    posts::table
        .filter(posts::user_id.eq(user_id))
        .order(posts::id.desc())
        .select(posts::all_columns)
        .load::<Post>(conn)
        .map_err(Into::into)
}

pub fn create_comment(
    conn: &SqliteConnection,
    user_id: i32,
    post_id: i32,
    body: &str,
) -> Result<Comment> {
    conn.transaction(|| {
        diesel::insert_into(comments::table)
            .values((
                comments::user_id.eq(user_id),
                comments::post_id.eq(post_id),
                comments::body.eq(body),
            ))
            .execute(conn)?;

        comments::table
            .order(comments::id.desc())
            .select(comments::all_columns)
            .first(conn)
            .map_err(Into::into)
    })
}

pub fn post_comments(conn: &SqliteConnection, post_id: i32) -> Result<Vec<(Comment, User)>> {
    comments::table
        .filter(comments::post_id.eq(post_id))
        .inner_join(users::table)
        .select((comments::all_columns, (users::id, users::username)))
        .load::<(Comment, User)>(conn)
        .map_err(Into::into)
}

pub fn user_comments(
    conn: &SqliteConnection,
    user_id: i32,
) -> Result<Vec<(Comment, PostWithComment)>> {
    comments::table
        .filter(comments::user_id.eq(user_id))
        .inner_join(posts::table)
        .select((
            comments::all_columns,
            (posts::id, posts::title, posts::published),
        ))
        .load::<(Comment, PostWithComment)>(conn)
        .map_err(Into::into)
}

pub fn all_posts_with_comments_user(
    conn: &SqliteConnection,
) -> Result<Vec<((Post, User), Vec<(Comment, User)>)>> {
    let query = posts::table
        .order(posts::id.desc())
        .filter(posts::published.eq(true))
        .inner_join(users::table)
        .select((posts::all_columns, (users::id, users::username)));
    let posts_with_user = query.load::<(Post, User)>(conn)?;
    // We then use the unzip method on std::iter::Iterator which turns an iterator of pairs into a pair of iterators.
    // we turn Vec<(Post, User)> into (Vec<Post>, Vec<User>).
    let (posts, post_users): (Vec<_>, Vec<_>) = posts_with_user.into_iter().unzip();
    // To associate the comments into chunks indexed by the posts we use the grouped_by method provided by Diesel. Note this does not generate a GROUP BY statement in SQL rather it is just operating on the data structures in memory of already loaded data.
    let comments = Comment::belonging_to(&posts)
        .inner_join(users::table)
        .select((comments::all_columns, (users::id, users::username)))
        .load::<(Comment, User)>(conn)?
        .grouped_by(&posts);
    // we can use the zip method on iterator to take all of these vectors and combine them into the output format we were looking for
    Ok(posts.into_iter().zip(post_users).zip(comments).collect())
}

pub fn user_posts_with_comments(
    conn: &SqliteConnection,
    user_id: i32,
) -> Result<Vec<(Post, Vec<(Comment, User)>)>> {
    let posts = posts::table
        .filter(posts::user_id.eq(user_id))
        .order(posts::id.desc())
        .select(posts::all_columns)
        .load::<Post>(conn)?;

    let comments = Comment::belonging_to(&posts)
        .inner_join(users::table)
        .select((comments::all_columns, (users::id, users::username)))
        .load::<(Comment, User)>(conn)?
        .grouped_by(&posts);

    Ok(posts.into_iter().zip(comments).collect())
}
