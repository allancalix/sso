use crate::{
    schema::sso_user, DriverError, DriverResult, User, UserCreate, UserList, UserListFilter,
    UserListQuery, UserRead, UserUpdate,
};
use chrono::{DateTime, Utc};
use diesel::{pg::Pg, prelude::*};
use std::convert::TryInto;
use uuid::Uuid;

#[derive(Debug, Identifiable, Queryable)]
#[table_name = "sso_user"]
#[primary_key(id)]
pub struct ModelUser {
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    id: Uuid,
    is_enabled: bool,
    name: String,
    email: String,
    locale: String,
    timezone: String,
    password_allow_reset: bool,
    password_require_update: bool,
    password_hash: Option<String>,
}

impl From<ModelUser> for User {
    fn from(user: ModelUser) -> Self {
        Self {
            created_at: user.created_at,
            updated_at: user.updated_at,
            id: user.id,
            is_enabled: user.is_enabled,
            name: user.name,
            email: user.email,
            locale: user.locale,
            timezone: user.timezone,
            password_allow_reset: user.password_allow_reset,
            password_require_update: user.password_require_update,
            password_hash: user.password_hash,
        }
    }
}

#[derive(Debug, Insertable)]
#[table_name = "sso_user"]
struct ModelUserInsert<'a> {
    created_at: &'a DateTime<Utc>,
    updated_at: &'a DateTime<Utc>,
    id: &'a Uuid,
    is_enabled: bool,
    name: &'a str,
    email: &'a str,
    locale: &'a str,
    timezone: &'a str,
    password_allow_reset: bool,
    password_require_update: bool,
    password_hash: Option<&'a str>,
}

#[derive(AsChangeset)]
#[table_name = "sso_user"]
struct ModelUserUpdate<'a> {
    updated_at: &'a DateTime<Utc>,
    is_enabled: Option<bool>,
    name: Option<&'a str>,
    email: Option<&'a str>,
    locale: Option<&'a str>,
    timezone: Option<&'a str>,
    password_allow_reset: Option<bool>,
    password_require_update: Option<bool>,
    password_hash: Option<&'a str>,
}

impl ModelUser {
    pub fn list(conn: &PgConnection, list: &UserList) -> DriverResult<Vec<User>> {
        match &list.query {
            UserListQuery::IdGt(gt) => Self::list_where_id_gt(conn, &gt, &list.filter),
            UserListQuery::IdLt(lt) => Self::list_where_id_lt(conn, &lt, &list.filter),
            UserListQuery::NameGe(name_ge, offset_id) => {
                Self::list_where_name_ge(conn, &name_ge, &offset_id, &list.filter)
            }
            UserListQuery::NameLe(name_le, offset_id) => {
                Self::list_where_name_le(conn, &name_le, &offset_id, &list.filter)
            }
        }
        .map(|x| x.into_iter().map(|x| x.into()).collect())
    }

    pub fn create(conn: &PgConnection, create: &UserCreate) -> DriverResult<User> {
        let user = Self::read_email(conn, &create.email)?;
        if user.is_some() {
            return Err(DriverError::UserEmailConstraint);
        }

        let now = Utc::now();
        let id = Uuid::new_v4();
        let value = ModelUserInsert {
            created_at: &now,
            updated_at: &now,
            id: &id,
            is_enabled: create.is_enabled,
            name: &create.name,
            email: &create.email,
            locale: &create.locale,
            timezone: &create.timezone,
            password_allow_reset: create.password_allow_reset,
            password_require_update: create.password_require_update,
            password_hash: create.password_hash.as_ref().map(|x| &**x),
        };
        diesel::insert_into(sso_user::table)
            .values(&value)
            .get_result::<ModelUser>(conn)
            .map_err(Into::into)
            .map(Into::into)
    }

    pub fn read(conn: &PgConnection, read: &UserRead) -> DriverResult<Option<User>> {
        match read {
            UserRead::Id(id) => Self::read_id(conn, id),
            UserRead::Email(email) => Self::read_email(conn, email),
        }
        .map(|r| r.map(|u| u.into()))
    }

    pub fn update(conn: &PgConnection, update: &UserUpdate) -> DriverResult<User> {
        let now = Utc::now();
        let value = ModelUserUpdate {
            updated_at: &now,
            is_enabled: update.is_enabled,
            name: update.name.as_ref().map(|x| &**x),
            email: update.email.as_ref().map(|x| &**x),
            locale: update.locale.as_ref().map(|x| &**x),
            timezone: update.timezone.as_ref().map(|x| &**x),
            password_allow_reset: update.password_allow_reset,
            password_require_update: update.password_require_update,
            password_hash: update.password_hash.as_ref().map(|x| &**x),
        };
        diesel::update(sso_user::table.filter(sso_user::dsl::id.eq(update.id)))
            .set(&value)
            .get_result::<ModelUser>(conn)
            .map_err(Into::into)
            .map(Into::into)
    }

    pub fn delete(conn: &PgConnection, id: &Uuid) -> DriverResult<usize> {
        diesel::delete(sso_user::table.filter(sso_user::dsl::id.eq(id)))
            .execute(conn)
            .map_err(Into::into)
    }

    fn list_where_id_gt(
        conn: &PgConnection,
        gt: &Uuid,
        filter: &UserListFilter,
    ) -> DriverResult<Vec<ModelUser>> {
        let mut query = sso_user::table.into_boxed();
        query = Self::boxed_query_filter(query, filter);

        query
            .filter(sso_user::dsl::id.gt(gt))
            .limit(filter.limit)
            .order(sso_user::dsl::id.asc())
            .load::<ModelUser>(conn)
            .map_err(DriverError::DieselResult)
    }

    fn list_where_id_lt(
        conn: &PgConnection,
        lt: &Uuid,
        filter: &UserListFilter,
    ) -> DriverResult<Vec<ModelUser>> {
        let mut query = sso_user::table.into_boxed();
        query = Self::boxed_query_filter(query, filter);

        query
            .filter(sso_user::dsl::id.lt(lt))
            .limit(filter.limit)
            .order(sso_user::dsl::id.desc())
            .load::<ModelUser>(conn)
            .map_err(DriverError::DieselResult)
            .map(|mut x| {
                x.reverse();
                x
            })
    }

    fn list_where_name_ge(
        conn: &PgConnection,
        name_ge: &str,
        offset_id: &Option<Uuid>,
        filter: &UserListFilter,
    ) -> DriverResult<Vec<ModelUser>> {
        let offset: i64 = if offset_id.is_some() { 1 } else { 0 };
        Self::list_where_name_ge_inner(conn, name_ge, offset, filter).and_then(|res| {
            if let Some(offset_id) = offset_id {
                for (i, user) in res.iter().enumerate() {
                    if &user.id == offset_id {
                        let offset: i64 = (i + 1).try_into().unwrap();
                        return Self::list_where_name_ge_inner(conn, name_ge, offset, filter);
                    }
                }
            }
            Ok(res)
        })
    }

    fn list_where_name_ge_inner(
        conn: &PgConnection,
        name_ge: &str,
        offset: i64,
        filter: &UserListFilter,
    ) -> DriverResult<Vec<ModelUser>> {
        let mut query = sso_user::table.into_boxed();
        query = Self::boxed_query_filter(query, filter);

        query
            .filter(sso_user::dsl::name.ge(name_ge))
            .limit(filter.limit)
            .offset(offset)
            .order(sso_user::dsl::name.asc())
            .load::<ModelUser>(conn)
            .map_err(DriverError::DieselResult)
    }

    fn list_where_name_le(
        conn: &PgConnection,
        name_le: &str,
        offset_id: &Option<Uuid>,
        filter: &UserListFilter,
    ) -> DriverResult<Vec<ModelUser>> {
        let offset: i64 = if offset_id.is_some() { 1 } else { 0 };
        Self::list_where_name_le_inner(conn, name_le, offset, filter).and_then(|mut res| {
            if let Some(offset_id) = offset_id {
                for (i, user) in res.iter().enumerate() {
                    if &user.id == offset_id {
                        let offset: i64 = (i + 1).try_into().unwrap();
                        return Self::list_where_name_le_inner(conn, name_le, offset, filter);
                    }
                }
            }
            res.reverse();
            Ok(res)
        })
    }

    fn list_where_name_le_inner(
        conn: &PgConnection,
        name_le: &str,
        offset: i64,
        filter: &UserListFilter,
    ) -> DriverResult<Vec<ModelUser>> {
        let mut query = sso_user::table.into_boxed();
        query = Self::boxed_query_filter(query, filter);

        query
            .filter(sso_user::dsl::name.le(name_le))
            .limit(filter.limit)
            .offset(offset)
            .order(sso_user::dsl::name.desc())
            .load::<ModelUser>(conn)
            .map_err(DriverError::DieselResult)
    }

    fn read_id(conn: &PgConnection, id: &Uuid) -> DriverResult<Option<ModelUser>> {
        sso_user::table
            .filter(sso_user::dsl::id.eq(id))
            .get_result::<ModelUser>(conn)
            .optional()
            .map_err(DriverError::DieselResult)
    }

    fn read_email(conn: &PgConnection, email: &str) -> DriverResult<Option<ModelUser>> {
        sso_user::table
            .filter(sso_user::dsl::email.eq(email))
            .get_result::<ModelUser>(conn)
            .optional()
            .map_err(DriverError::DieselResult)
    }

    fn boxed_query_filter<'a>(
        mut query: sso_user::BoxedQuery<'a, Pg>,
        filter: &'a UserListFilter,
    ) -> sso_user::BoxedQuery<'a, Pg> {
        use diesel::dsl::any;

        if let Some(id) = &filter.id {
            let id: Vec<Uuid> = id.iter().copied().collect();
            query = query.filter(sso_user::dsl::id.eq(any(id)));
        }
        if let Some(email) = &filter.email {
            let email: Vec<String> = email.to_vec();
            query = query.filter(sso_user::dsl::email.eq(any(email)));
        }

        query
    }
}
