// ═══════════════════════════════════════════════════════════════
//  微信小程序登录错误 (模块 02)
//  用户错误: 40201~40299
//  内部错误: 50201~50299
// ═══════════════════════════════════════════════════════════════

// ── 小程序登录 ──────────────────────────────────────────────

/// 构建请求 URL 失败 (50201)
#[inline]
pub fn mini_url() -> crate::Error {
    crate::Error::custom(50201, "WeChat mini login URL build failed")
}

/// 发送请求失败 (50202)
#[inline]
pub fn mini_request() -> crate::Error {
    crate::Error::custom(50202, "WeChat mini login request failed")
}

/// 解析响应 JSON 失败 (50203)
#[inline]
pub fn mini_response() -> crate::Error {
    crate::Error::custom(50203, "WeChat mini login response parse failed")
}

/// 微信 API 返回业务错误 (40201)
#[inline]
pub fn mini_api(errmsg: &str) -> crate::Error {
    crate::Error::custom(40201, errmsg)
}

/// 反序列化登录响应失败 (50204)
#[inline]
pub fn mini_parse() -> crate::Error {
    crate::Error::custom(50204, "WeChat mini login response deserialize failed")
}

// ═══════════════════════════════════════════════════════════════
//  微信 APP 登录错误 (模块 03)
//  用户错误: 40301~40399
//  内部错误: 50301~50399
// ═══════════════════════════════════════════════════════════════

// ── APP 登录 ────────────────────────────────────────────────

/// 构建请求 URL 失败 (50301)
#[inline]
pub fn app_url() -> crate::Error {
    crate::Error::custom(50301, "WeChat APP login URL build failed")
}

/// 发送请求失败 (50302)
#[inline]
pub fn app_request() -> crate::Error {
    crate::Error::custom(50302, "WeChat APP login request failed")
}

/// 解析响应 JSON 失败 (50303)
#[inline]
pub fn app_response() -> crate::Error {
    crate::Error::custom(50303, "WeChat APP login response parse failed")
}

/// 微信 API 返回业务错误 (40301)
#[inline]
pub fn app_api(errmsg: &str) -> crate::Error {
    crate::Error::custom(40301, errmsg)
}

/// 反序列化登录响应失败 (50304)
#[inline]
pub fn app_parse() -> crate::Error {
    crate::Error::custom(50304, "WeChat APP login response deserialize failed")
}

// ═══════════════════════════════════════════════════════════════
//  微信网页登录错误 (模块 21)
//  用户错误: 42101~42199
//  内部错误: 52101~52199
// ═══════════════════════════════════════════════════════════════

// ── 网页登录 ────────────────────────────────────────────────

/// 构建授权 URL 失败 (52101)
#[inline]
pub fn web_url() -> crate::Error {
    crate::Error::custom(52101, "WeChat web login URL build failed")
}

/// 通过 code 换取 access_token 请求失败 (52102)
#[inline]
pub fn web_request() -> crate::Error {
    crate::Error::custom(52102, "WeChat web login request failed")
}

/// 解析 access_token 响应 JSON 失败 (52103)
#[inline]
pub fn web_response() -> crate::Error {
    crate::Error::custom(52103, "WeChat web login response parse failed")
}

/// 反序列化 access_token 响应失败 (52104)
#[inline]
pub fn web_parse() -> crate::Error {
    crate::Error::custom(52104, "WeChat web login response deserialize failed")
}

/// 微信 API 返回业务错误 (42101)
#[inline]
pub fn web_api(errmsg: &str) -> crate::Error {
    crate::Error::custom(42101, errmsg)
}

/// 刷新 access_token 请求失败 (52105)
#[inline]
pub fn web_refresh_request() -> crate::Error {
    crate::Error::custom(52105, "WeChat web refresh token request failed")
}

/// 解析刷新 access_token 响应失败 (52106)
#[inline]
pub fn web_refresh_response() -> crate::Error {
    crate::Error::custom(52106, "WeChat web refresh token response parse failed")
}

/// 检验 access_token 请求失败 (52107)
#[inline]
pub fn web_check_request() -> crate::Error {
    crate::Error::custom(52107, "WeChat web check token request failed")
}

/// 解析检验 access_token 响应失败 (52108)
#[inline]
pub fn web_check_response() -> crate::Error {
    crate::Error::custom(52108, "WeChat web check token response parse failed")
}

/// 获取用户信息请求失败 (52109)
#[inline]
pub fn web_userinfo_request() -> crate::Error {
    crate::Error::custom(52109, "WeChat web userinfo request failed")
}

/// 解析用户信息响应失败 (52110)
#[inline]
pub fn web_userinfo_response() -> crate::Error {
    crate::Error::custom(52110, "WeChat web userinfo response parse failed")
}

/// 缺少 code 参数 (42102)
#[inline]
pub fn web_no_code() -> crate::Error {
    crate::Error::custom(42102, "Missing code parameter")
}

/// 缺少 state 参数 (42103)
#[inline]
pub fn web_no_state() -> crate::Error {
    crate::Error::custom(42103, "Missing state parameter")
}

/// state 验证失败 (42104)
#[inline]
pub fn web_invalid_state() -> crate::Error {
    crate::Error::custom(42104, "Invalid or expired state")
}

/// 回调函数未注册 (52111)
#[inline]
pub fn web_no_callback() -> crate::Error {
    crate::Error::custom(52111, "WeChat web login callback not registered")
}

// ═══════════════════════════════════════════════════════════════
//  微信公众号网页授权错误 (模块 22)
//  用户错误: 42201~42299
//  内部错误: 52201~52299
// ═══════════════════════════════════════════════════════════════

/// 构建授权 URL 失败 (52201)
#[inline]
pub fn mp_url() -> crate::Error {
    crate::Error::custom(52201, "WeChat mp login URL build failed")
}

/// 通过 code 换取 access_token 请求失败 (52202)
#[inline]
pub fn mp_request() -> crate::Error {
    crate::Error::custom(52202, "WeChat mp login request failed")
}

/// 解析 access_token 响应 JSON 失败 (52203)
#[inline]
pub fn mp_response() -> crate::Error {
    crate::Error::custom(52203, "WeChat mp login response parse failed")
}

/// 反序列化 access_token 响应失败 (52204)
#[inline]
pub fn mp_parse() -> crate::Error {
    crate::Error::custom(52204, "WeChat mp login response deserialize failed")
}

/// 微信 API 返回业务错误 (42201)
#[inline]
pub fn mp_api(errmsg: &str) -> crate::Error {
    crate::Error::custom(42201, errmsg)
}

/// 刷新 access_token 请求失败 (52205)
#[inline]
pub fn mp_refresh_request() -> crate::Error {
    crate::Error::custom(52205, "WeChat mp refresh token request failed")
}

/// 解析刷新 access_token 响应失败 (52206)
#[inline]
pub fn mp_refresh_response() -> crate::Error {
    crate::Error::custom(52206, "WeChat mp refresh token response parse failed")
}

/// 检验 access_token 请求失败 (52207)
#[inline]
pub fn mp_check_request() -> crate::Error {
    crate::Error::custom(52207, "WeChat mp check token request failed")
}

/// 解析检验 access_token 响应失败 (52208)
#[inline]
pub fn mp_check_response() -> crate::Error {
    crate::Error::custom(52208, "WeChat mp check token response parse failed")
}

/// 获取用户信息请求失败 (52209)
#[inline]
pub fn mp_userinfo_request() -> crate::Error {
    crate::Error::custom(52209, "WeChat mp userinfo request failed")
}

/// 解析用户信息响应失败 (52210)
#[inline]
pub fn mp_userinfo_response() -> crate::Error {
    crate::Error::custom(52210, "WeChat mp userinfo response parse failed")
}

/// 缺少 code 参数 (42202)
#[inline]
pub fn mp_no_code() -> crate::Error {
    crate::Error::custom(42202, "Missing code parameter")
}

/// 回调函数未注册 (52211)
#[inline]
pub fn mp_no_callback() -> crate::Error {
    crate::Error::custom(52211, "WeChat mp login callback not registered")
}
