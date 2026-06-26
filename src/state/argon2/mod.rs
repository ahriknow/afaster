use argon2::{
    Algorithm, Argon2, Params, Version,
    password_hash::{
        self, PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng,
    },
};

// ═══════════════════════════════════════════════════════════════
//  Argon2 变体
// ═══════════════════════════════════════════════════════════════

/// Argon2 变体枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Argon2Variant {
    /// Argon2i - 抗侧信道攻击，适合密钥派生
    I,
    /// Argon2d - 抗 GPU 破解攻击，适合加密货币/纯数据保护
    D,
    /// Argon2id - 混合模式，推荐用于密码哈希（默认）
    #[default]
    ID,
}

// ═══════════════════════════════════════════════════════════════
//  Argon2 配置
// ═══════════════════════════════════════════════════════════════

/// Argon2 哈希参数配置
#[derive(Debug, Clone)]
pub struct Argon2Config {
    /// 变体类型
    pub variant: Argon2Variant,
    /// 内存成本 (KB)，默认 19456 (19 MB)
    pub memory_cost: u32,
    /// 迭代次数，默认 2
    pub iterations: u32,
    /// 并行度，默认 1
    pub parallelism: u32,
}

impl Default for Argon2Config {
    fn default() -> Self {
        Self {
            variant: Argon2Variant::ID,
            memory_cost: 19456, // 19 MB - OWASP 推荐
            iterations: 2,
            parallelism: 1,
        }
    }
}

impl Argon2Config {
    /// 创建新的配置
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置变体
    pub fn variant(mut self, variant: Argon2Variant) -> Self {
        self.variant = variant;
        self
    }

    /// 设置内存成本 (KB)
    pub fn memory_cost(mut self, memory_cost: u32) -> Self {
        self.memory_cost = memory_cost;
        self
    }

    /// 设置迭代次数
    pub fn iterations(mut self, iterations: u32) -> Self {
        self.iterations = iterations;
        self
    }

    /// 设置并行度
    pub fn parallelism(mut self, parallelism: u32) -> Self {
        self.parallelism = parallelism;
        self
    }
}

// ═══════════════════════════════════════════════════════════════
//  Argon2 哈希器
// ═══════════════════════════════════════════════════════════════

/// Argon2 密码哈希工具
///
/// 支持 Argon2i、Argon2d、Argon2id 三种变体。
///
/// # 示例
///
/// ```rust,no_run
/// use afaster::argon2::{Argon2Hasher, Argon2Variant, Argon2Config};
///
/// // 使用默认配置 (Argon2id)
/// let hasher = Argon2Hasher::new();
/// let hash = hasher.hash("my_password").unwrap();
/// assert!(hasher.verify("my_password", &hash).unwrap());
/// assert!(!hasher.verify("wrong_password", &hash).unwrap());
///
/// // 自定义配置
/// let hasher = Argon2Hasher::with_config(Argon2Config {
///     variant: Argon2Variant::I,
///     memory_cost: 65536,  // 64 MB
///     iterations: 3,
///     parallelism: 4,
/// });
/// let hash = hasher.hash("my_password").unwrap();
/// ```
#[derive(Clone)]
pub struct Argon2Hasher {
    config: Argon2Config,
}

impl Argon2Hasher {
    /// 使用默认配置创建哈希器 (Argon2id, 19MB, 2 iterations, 1 parallelism)
    pub fn new() -> Self {
        Self {
            config: Argon2Config::default(),
        }
    }

    /// 使用自定义配置创建哈希器
    pub fn with_config(config: Argon2Config) -> Self {
        Self { config }
    }

    fn algorithm(&self) -> Algorithm {
        match self.config.variant {
            Argon2Variant::I => Algorithm::Argon2i,
            Argon2Variant::D => Algorithm::Argon2d,
            Argon2Variant::ID => Algorithm::Argon2id,
        }
    }

    fn argon2(&self) -> Result<Argon2<'_>, password_hash::Error> {
        let params = Params::new(
            self.config.memory_cost,
            self.config.iterations,
            self.config.parallelism,
            None,
        )?;
        Ok(Argon2::new(self.algorithm(), Version::V0x13, params))
    }

    /// 对密码进行哈希
    ///
    /// 自动生成随机 16 字节 salt，返回 PHC 格式哈希字符串。
    ///
    /// # 参数
    /// - `password`: 明文密码
    ///
    /// # 返回
    /// - `crate::Result<String>`: PHC 格式的哈希字符串
    ///
    /// # 示例输出
    /// `$argon2id$v=19$m=19456,t=2,p=1$c2FsdHNhbHRzYWx0$hash...`
    pub fn hash(&self, password: &str) -> crate::Result<String> {
        let argon2 = self.argon2().map_err(|_e| {
            #[cfg(feature = "log")]
            tracing::error!({ code = 52401, msg = "Argon2 config error" }, "{}", _e);
            crate::Error::custom(52401, "Argon2 config error")
        })?;

        let salt = SaltString::generate(&mut OsRng);
        let hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|_e| {
                #[cfg(feature = "log")]
                tracing::error!({ code = 52402, msg = "Argon2 hash error" }, "{}", _e);
                crate::Error::custom(52402, "Argon2 hash failed")
            })?
            .to_string();

        Ok(hash)
    }

    /// 使用指定 salt 对密码进行哈希
    ///
    /// # 参数
    /// - `password`: 明文密码
    /// - `salt`: Base64 编码的 salt（16 字节 = 22 字符）
    ///
    /// # 返回
    /// - `crate::Result<String>`: PHC 格式的哈希字符串
    pub fn hash_with_salt(&self, password: &str, salt: &str) -> crate::Result<String> {
        let argon2 = self.argon2().map_err(|_e| {
            #[cfg(feature = "log")]
            tracing::error!({ code = 52401, msg = "Argon2 config error" }, "{}", _e);
            crate::Error::custom(52401, "Argon2 config error")
        })?;

        let salt = SaltString::encode_b64(salt.as_bytes()).map_err(|_e| {
            #[cfg(feature = "log")]
            tracing::error!({ code = 52403, msg = "Argon2 salt error" }, "{}", _e);
            crate::Error::custom(52403, "Invalid salt")
        })?;

        let hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|_e| {
                #[cfg(feature = "log")]
                tracing::error!({ code = 52402, msg = "Argon2 hash error" }, "{}", _e);
                crate::Error::custom(52402, "Argon2 hash failed")
            })?
            .to_string();

        Ok(hash)
    }

    /// 验证密码是否匹配哈希值
    ///
    /// # 参数
    /// - `password`: 明文密码
    /// - `hash`: PHC 格式的哈希字符串
    ///
    /// # 返回
    /// - `crate::Result<bool>`: true 表示匹配
    pub fn verify(&self, password: &str, hash: &str) -> crate::Result<bool> {
        let parsed_hash = PasswordHash::new(hash).map_err(|_e| {
            #[cfg(feature = "log")]
            tracing::error!({ code = 52404, msg = "Argon2 hash parse error" }, "{}", _e);
            crate::Error::custom(52404, "Invalid hash format")
        })?;

        let argon2 = self.argon2().map_err(|_e| {
            #[cfg(feature = "log")]
            tracing::error!({ code = 52401, msg = "Argon2 config error" }, "{}", _e);
            crate::Error::custom(52401, "Argon2 config error")
        })?;

        let result = argon2
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok();

        Ok(result)
    }

    /// 从哈希字符串中解析出变体信息
    ///
    /// # 参数
    /// - `hash`: PHC 格式的哈希字符串
    ///
    /// # 返回
    /// - `crate::Result<Argon2HashInfo>`: 哈希参数信息
    pub fn parse_hash(&self, hash: &str) -> crate::Result<Argon2HashInfo> {
        let parsed = PasswordHash::new(hash).map_err(|_e| {
            #[cfg(feature = "log")]
            tracing::error!({ code = 52404, msg = "Argon2 hash parse error" }, "{}", _e);
            crate::Error::custom(52404, "Invalid hash format")
        })?;

        let algorithm = parsed.algorithm.as_str().to_string();
        let variant = match algorithm.as_str() {
            "argon2i" => Argon2Variant::I,
            "argon2d" => Argon2Variant::D,
            "argon2id" => Argon2Variant::ID,
            _ => Argon2Variant::ID,
        };

        let version = parsed.version.unwrap_or(0x13);

        // 从 PHC 字符串中解析参数: $argon2id$v=19$m=19456,t=2,p=1$salt$hash
        let params_str = parsed.params.as_str();
        let mut memory_cost = 0u32;
        let mut iterations = 0u32;
        let mut parallelism = 0u32;
        for part in params_str.split(',') {
            if let Some(val) = part.strip_prefix("m=") {
                memory_cost = val.parse().unwrap_or(0);
            } else if let Some(val) = part.strip_prefix("t=") {
                iterations = val.parse().unwrap_or(0);
            } else if let Some(val) = part.strip_prefix("p=") {
                parallelism = val.parse().unwrap_or(0);
            }
        }

        Ok(Argon2HashInfo {
            variant,
            version,
            memory_cost,
            iterations,
            parallelism,
        })
    }
}

impl Default for Argon2Hasher {
    fn default() -> Self {
        Self::new()
    }
}

/// Argon2 哈希参数信息（从哈希字符串中解析）
#[derive(Debug, Clone)]
pub struct Argon2HashInfo {
    pub variant: Argon2Variant,
    pub version: u32,
    pub memory_cost: u32,
    pub iterations: u32,
    pub parallelism: u32,
}

// ═══════════════════════════════════════════════════════════════
//  便捷函数
// ═══════════════════════════════════════════════════════════════

/// 使用默认配置 (Argon2id) 对密码进行哈希
pub fn hash(password: &str) -> crate::Result<String> {
    Argon2Hasher::new().hash(password)
}

/// 使用默认配置验证密码
pub fn verify(password: &str, hash: &str) -> crate::Result<bool> {
    Argon2Hasher::new().verify(password, hash)
}

/// 使用指定变体对密码进行哈希
pub fn hash_with_variant(password: &str, variant: Argon2Variant) -> crate::Result<String> {
    Argon2Hasher::with_config(Argon2Config::new().variant(variant)).hash(password)
}
