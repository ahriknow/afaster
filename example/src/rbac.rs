use std::{future::Future, pin::Pin};

use afaster::{
    Result,
    orm::Orm,
    rbac::{RbacStore, Role},
};
use afastorm::Field;

use crate::model::entities::{RbacRole, RbacRolePermission, RbacUserRole};

#[derive(Clone)]
pub struct SqliteRbacStore {
    orm: Orm,
}

impl SqliteRbacStore {
    pub fn new(orm: Orm) -> Self {
        Self { orm }
    }

    async fn get_role_id_inner(&self, name: &str) -> Result<i64> {
        let role = self
            .orm
            .repo::<RbacRole>()
            .map_err(db_error)?
            .first_where(Field::new("name").eq(name))
            .await
            .map_err(db_error)?
            .ok_or_else(|| afaster::Error::custom(42801, format!("角色不存在: {name}")))?;
        role.id
            .ok_or_else(|| afaster::Error::custom(52805, "角色记录没有主键"))
    }
}

impl RbacStore for SqliteRbacStore {
    fn check_permission(
        &self,
        user_id: i64,
        permission_code: &str,
    ) -> Pin<Box<dyn Future<Output = Result<bool>> + Send + '_>> {
        let permission_code = permission_code.to_string();
        Box::pin(async move {
            let query = self.orm.repo::<RbacUserRole>().map_err(db_error)?;
            let statement = query
                .query()
                .filter(Field::new("user_id").eq(user_id))
                .build()
                .map_err(db_error)?;
            let roles = self
                .orm
                .fetch_all::<RbacUserRole>(&statement)
                .await
                .map_err(db_error)?;
            let permissions = self.orm.repo::<RbacRolePermission>().map_err(db_error)?;
            for role in roles {
                if permissions
                    .first_where(
                        Field::new("role_id")
                            .eq(role.role_id)
                            .and(Field::new("permission_code").eq(permission_code.as_str())),
                    )
                    .await
                    .map_err(db_error)?
                    .is_some()
                {
                    return Ok(true);
                }
            }
            Ok(false)
        })
    }

    fn get_user_permissions(
        &self,
        user_id: i64,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<String>>> + Send + '_>> {
        Box::pin(async move {
            let statement = self
                .orm
                .repo::<RbacUserRole>()
                .map_err(db_error)?
                .query()
                .filter(Field::new("user_id").eq(user_id))
                .build()
                .map_err(db_error)?;
            let roles = self
                .orm
                .fetch_all::<RbacUserRole>(&statement)
                .await
                .map_err(db_error)?;
            let permissions = self.orm.repo::<RbacRolePermission>().map_err(db_error)?;
            let mut codes = std::collections::BTreeSet::new();
            for role in roles {
                let statement = permissions
                    .query()
                    .filter(Field::new("role_id").eq(role.role_id))
                    .build()
                    .map_err(db_error)?;
                for permission in self
                    .orm
                    .fetch_all::<RbacRolePermission>(&statement)
                    .await
                    .map_err(db_error)?
                {
                    codes.insert(permission.permission_code);
                }
            }
            Ok(codes.into_iter().collect())
        })
    }

    fn get_user_roles(
        &self,
        user_id: i64,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Role>>> + Send + '_>> {
        Box::pin(async move {
            let statement = self
                .orm
                .repo::<RbacUserRole>()
                .map_err(db_error)?
                .query()
                .filter(Field::new("user_id").eq(user_id))
                .build()
                .map_err(db_error)?;
            let links = self
                .orm
                .fetch_all::<RbacUserRole>(&statement)
                .await
                .map_err(db_error)?;
            let roles = self.orm.repo::<RbacRole>().map_err(db_error)?;
            let mut result = Vec::new();
            for link in links {
                if let Some(role) = roles.find_by_id(link.role_id).await.map_err(db_error)? {
                    result.push(role_to_rbac(role)?);
                }
            }
            Ok(result)
        })
    }

    fn assign_role(
        &self,
        user_id: i64,
        role_name: &str,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
        let role_name = role_name.to_string();
        Box::pin(async move {
            let role_id = self.get_role_id_inner(&role_name).await?;
            let repository = self.orm.repo::<RbacUserRole>().map_err(db_error)?;
            let existing = repository
                .first_where(
                    Field::new("user_id")
                        .eq(user_id)
                        .and(Field::new("role_id").eq(role_id)),
                )
                .await
                .map_err(db_error)?;
            if existing.is_none() {
                let mut link = RbacUserRole { user_id, role_id };
                repository.insert(&mut link).await.map_err(db_error)?;
            }
            Ok(())
        })
    }

    fn remove_role(
        &self,
        user_id: i64,
        role_name: &str,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
        let role_name = role_name.to_string();
        Box::pin(async move {
            let role_id = self.get_role_id_inner(&role_name).await?;
            self.orm
                .repo::<RbacUserRole>()
                .map_err(db_error)?
                .delete_where(
                    Field::new("user_id")
                        .eq(user_id)
                        .and(Field::new("role_id").eq(role_id)),
                )
                .await
                .map_err(db_error)?;
            Ok(())
        })
    }

    fn create_role(
        &self,
        name: &str,
        display_name: &str,
        permissions: &[&str],
    ) -> Pin<Box<dyn Future<Output = Result<i64>> + Send + '_>> {
        let name = name.to_string();
        let display_name = display_name.to_string();
        let permissions: Vec<String> = permissions.iter().map(|s| s.to_string()).collect();
        Box::pin(async move {
            let now = chrono::Local::now().naive_local();
            let mut role = RbacRole {
                id: None,
                name,
                display_name,
                created_at: now,
            };
            self.orm
                .repo::<RbacRole>()
                .map_err(db_error)?
                .insert(&mut role)
                .await
                .map_err(db_error)?;
            let role_id = role
                .id
                .ok_or_else(|| afaster::Error::custom(52805, "创建角色未返回 ID"))?;

            for perm_code in &permissions {
                let repository = self.orm.repo::<RbacRolePermission>().map_err(db_error)?;
                if repository
                    .first_where(
                        Field::new("role_id")
                            .eq(role_id)
                            .and(Field::new("permission_code").eq(perm_code.as_str())),
                    )
                    .await
                    .map_err(db_error)?
                    .is_none()
                {
                    let mut permission = RbacRolePermission {
                        role_id,
                        permission_code: perm_code.clone(),
                    };
                    repository.insert(&mut permission).await.map_err(db_error)?;
                }
            }

            Ok(role_id)
        })
    }

    fn delete_role(&self, name: &str) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
        let name = name.to_string();
        Box::pin(async move {
            let role_id = self.get_role_id_inner(&name).await?;
            self.orm
                .repo::<RbacRolePermission>()
                .map_err(db_error)?
                .delete_where(Field::new("role_id").eq(role_id))
                .await
                .map_err(db_error)?;
            self.orm
                .repo::<RbacUserRole>()
                .map_err(db_error)?
                .delete_where(Field::new("role_id").eq(role_id))
                .await
                .map_err(db_error)?;
            self.orm
                .repo::<RbacRole>()
                .map_err(db_error)?
                .delete(role_id)
                .await
                .map_err(db_error)?;
            Ok(())
        })
    }

    fn get_role(&self, name: &str) -> Pin<Box<dyn Future<Output = Result<Role>> + Send + '_>> {
        let name = name.to_string();
        Box::pin(async move {
            let role = self
                .orm
                .repo::<RbacRole>()
                .map_err(db_error)?
                .first_where(Field::new("name").eq(name.as_str()))
                .await
                .map_err(db_error)?
                .ok_or_else(|| afaster::Error::custom(42801, format!("角色不存在: {name}")))?;
            role_to_rbac(role)
        })
    }

    fn list_roles(&self) -> Pin<Box<dyn Future<Output = Result<Vec<Role>>> + Send + '_>> {
        Box::pin(async move {
            let roles = self.orm.repo::<RbacRole>().map_err(db_error)?;
            let statement = roles
                .query()
                .order_by(RbacRole::id.asc())
                .build()
                .map_err(db_error)?;
            let rows = roles.fetch_all(&statement).await.map_err(db_error)?;
            rows.into_iter().map(role_to_rbac).collect()
        })
    }

    fn get_role_permissions(
        &self,
        role_id: i64,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<String>>> + Send + '_>> {
        Box::pin(async move {
            let statement = self
                .orm
                .repo::<RbacRolePermission>()
                .map_err(db_error)?
                .query()
                .filter(Field::new("role_id").eq(role_id))
                .build()
                .map_err(db_error)?;
            let rows = self
                .orm
                .fetch_all::<RbacRolePermission>(&statement)
                .await
                .map_err(db_error)?;
            Ok(rows.into_iter().map(|row| row.permission_code).collect())
        })
    }

    fn get_role_id(&self, name: &str) -> Pin<Box<dyn Future<Output = Result<i64>> + Send + '_>> {
        let name = name.to_string();
        Box::pin(async move { self.get_role_id_inner(&name).await })
    }
}

fn role_to_rbac(role: RbacRole) -> Result<Role> {
    Ok(Role {
        id: role
            .id
            .ok_or_else(|| afaster::Error::custom(52805, "角色记录没有主键"))?,
        name: role.name,
        display_name: role.display_name,
        created_at: crate::model::entities::format_datetime(role.created_at),
    })
}

fn db_error(error: impl std::fmt::Display) -> afaster::Error {
    afaster::Error::custom(52803, format!("ORM: {error}"))
}
