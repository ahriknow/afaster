#![allow(dead_code)]

// ═══════════════════════════════════════════════════════════════
//  微信公众号管理错误 (模块 23)
//  用户错误: 42301~42399
//  内部错误: 52301~52399
// ═══════════════════════════════════════════════════════════════

/// 微信 API 返回业务错误 (42301)
#[inline]
pub fn api_error(errmsg: &str) -> crate::Error {
    crate::Error::custom(42301, errmsg)
}

/// access_token 未配置 (42302)
#[inline]
pub fn token_not_configured() -> crate::Error {
    crate::Error::custom(42302, "access_token not configured")
}

/// 获取 access_token 请求失败 (52301)
#[inline]
pub fn token_request() -> crate::Error {
    crate::Error::custom(52301, "access_token request failed")
}

/// 解析 access_token 响应失败 (52302)
#[inline]
pub fn token_response() -> crate::Error {
    crate::Error::custom(52302, "access_token response parse failed")
}

/// 发送模板消息请求失败 (52303)
#[inline]
pub fn template_send_request() -> crate::Error {
    crate::Error::custom(52303, "template message send request failed")
}

/// 解析模板消息响应失败 (52304)
#[inline]
pub fn template_send_response() -> crate::Error {
    crate::Error::custom(52304, "template message send response parse failed")
}

/// 创建菜单请求失败 (52305)
#[inline]
pub fn menu_create_request() -> crate::Error {
    crate::Error::custom(52305, "menu create request failed")
}

/// 解析菜单响应失败 (52306)
#[inline]
pub fn menu_response() -> crate::Error {
    crate::Error::custom(52306, "menu response parse failed")
}

/// 删除菜单请求失败 (52307)
#[inline]
pub fn menu_delete_request() -> crate::Error {
    crate::Error::custom(52307, "menu delete request failed")
}

/// 素材操作请求失败 (52308)
#[inline]
pub fn material_request() -> crate::Error {
    crate::Error::custom(52308, "material request failed")
}

/// 解析素材响应失败 (52309)
#[inline]
pub fn material_response() -> crate::Error {
    crate::Error::custom(52309, "material response parse failed")
}

/// 获取菜单请求失败 (52310)
#[inline]
pub fn menu_get_request() -> crate::Error {
    crate::Error::custom(52310, "menu get request failed")
}

/// 个性化菜单请求失败 (52311)
#[inline]
pub fn menu_conditional_request() -> crate::Error {
    crate::Error::custom(52311, "conditional menu request failed")
}

// ═══════════════════════════════════════════════════════════════
//  公众号网页授权错误 (原模块 22)
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
