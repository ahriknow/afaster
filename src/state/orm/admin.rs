use afast::{
    Body, Header, Html, HttpResult, Json, Param, Query, State, delete, get, patch, post, service,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value as JsonValue, json};
use std::{
    collections::{HashMap, HashSet},
    ops::Deref,
    sync::Arc,
};

use afastorm::{
    Error as OrmError, FieldInfo, FieldType, ModelInfo, ModelRegistry, Orm, OrmDecode, OrmRow,
    Statement, Value,
};

pub type AdminTokenGenerator = Arc<dyn Fn(&str) -> Option<String> + Send + Sync>;
pub type AdminTokenVerifier = Arc<dyn Fn(&str) -> bool + Send + Sync>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AdminFieldInfo {
    pub rust_name: &'static str,
    pub label: Option<&'static str>,
    pub label_lang: Option<&'static str>,
    pub order: Option<u32>,
    pub list: bool,
    pub detail: bool,
    pub create: bool,
    pub update: bool,
    pub filter: bool,
    pub search: bool,
    pub readonly: bool,
    pub hidden: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AdminModelInfo {
    pub rust_name: &'static str,
    pub enabled: bool,
    pub label: Option<&'static str>,
    pub label_lang: Option<&'static str>,
    pub list: bool,
    pub detail: bool,
    pub create: bool,
    pub update: bool,
    pub delete: bool,
    pub fields: &'static [AdminFieldInfo],
}

pub trait AFasterAdmin: Send + Sync + 'static {
    const ADMIN_INFO: AdminModelInfo;
}

#[derive(Clone, Debug, Default)]
pub struct AdminModelRegistry {
    models: Vec<&'static AdminModelInfo>,
}

impl AdminModelRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register<M: AFasterAdmin>(mut self) -> Self {
        if !self
            .models
            .iter()
            .any(|model| model.rust_name == M::ADMIN_INFO.rust_name)
        {
            self.models.push(&M::ADMIN_INFO);
        }
        self
    }

    pub(crate) fn has_enabled(&self) -> bool {
        self.models.iter().any(|model| model.enabled)
    }
}

#[derive(Clone, Copy)]
struct AdminField {
    database: FieldInfo,
    admin: AdminFieldInfo,
}

impl Deref for AdminField {
    type Target = FieldInfo;

    fn deref(&self) -> &Self::Target {
        &self.database
    }
}

struct AdminModel {
    database: &'static ModelInfo,
    admin: &'static AdminModelInfo,
    fields: Vec<AdminField>,
}

impl Deref for AdminModel {
    type Target = ModelInfo;

    fn deref(&self) -> &Self::Target {
        self.database
    }
}

impl AdminModel {
    fn label(&self, language: &str, i18ns: &[(String, HashMap<String, String>)]) -> String {
        localized_label(
            self.admin.label,
            self.admin.label_lang,
            self.database.table_name,
            language,
            i18ns,
        )
    }
}

fn localized_label(
    label: Option<&str>,
    label_lang: Option<&str>,
    fallback: &str,
    language: &str,
    i18ns: &[(String, HashMap<String, String>)],
) -> String {
    if let Some(label) = label {
        return label.to_owned();
    }
    if let Some(key) = label_lang {
        return i18ns
            .iter()
            .find(|(code, _)| code == language)
            .and_then(|(_, messages)| messages.get(key))
            .or_else(|| i18ns.first().and_then(|(_, messages)| messages.get(key)))
            .cloned()
            .unwrap_or_else(|| fallback.to_owned());
    }
    fallback.to_owned()
}

#[derive(Clone)]
pub struct AdminAuth {
    token: Option<Arc<str>>,
    token_generator: Option<AdminTokenGenerator>,
    token_verifier: Option<AdminTokenVerifier>,
    logo_letter: Arc<str>,
    brand_text: Arc<str>,
    models: Arc<Vec<AdminModel>>,
    i18ns: Arc<Vec<(String, HashMap<String, String>)>>,
}

impl AdminAuth {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        token: Option<String>,
        token_generator: Option<AdminTokenGenerator>,
        token_verifier: Option<AdminTokenVerifier>,
        logo_letter: String,
        brand_text: String,
        i18ns: Vec<(String, HashMap<String, String>)>,
        admin_registry: &AdminModelRegistry,
        model_registry: &ModelRegistry,
    ) -> crate::Result<Self> {
        if token_generator.is_some() != token_verifier.is_some() {
            return Err(crate::Error::custom(
                50001,
                "ORM admin token generator and verifier must be configured together",
            ));
        }
        let token = token
            .filter(|value| !value.trim().is_empty())
            .map(Arc::from);
        let i18ns = if i18ns.is_empty() {
            vec![("zh-CN".to_owned(), HashMap::new())]
        } else {
            i18ns
        };
        let mut codes = HashSet::new();
        if i18ns
            .iter()
            .any(|(code, _)| code.trim().is_empty() || !codes.insert(code.as_str()))
        {
            return Err(crate::Error::custom(
                50001,
                "ORM admin i18n language codes must be non-empty and unique",
            ));
        }
        let mut models = Vec::new();
        for admin in &admin_registry.models {
            if !admin.enabled {
                continue;
            }
            let database = model_registry
                .models()
                .iter()
                .find(|model| model.info.rust_name == admin.rust_name)
                .map(|model| model.info)
                .ok_or_else(|| {
                    crate::Error::custom(
                        50001,
                        format!(
                            "admin model {} is not registered in the ORM",
                            admin.rust_name
                        ),
                    )
                })?;
            let mut fields = Vec::new();
            for admin_field in admin.fields {
                let database_field = database
                    .fields
                    .iter()
                    .find(|field| field.rust_name == admin_field.rust_name)
                    .copied()
                    .ok_or_else(|| {
                        crate::Error::custom(
                            50001,
                            format!(
                                "admin field {}.{} is not registered in ORM metadata",
                                admin.rust_name, admin_field.rust_name
                            ),
                        )
                    })?;
                fields.push(AdminField {
                    database: database_field,
                    admin: *admin_field,
                });
            }
            models.push(AdminModel {
                database,
                admin,
                fields,
            });
        }
        Ok(Self {
            token,
            token_generator,
            token_verifier,
            logo_letter: Arc::from(if logo_letter.trim().is_empty() {
                "A"
            } else {
                logo_letter.as_str()
            }),
            brand_text: Arc::from(if brand_text.trim().is_empty() {
                "Afast ORM"
            } else {
                brand_text.as_str()
            }),
            models: Arc::new(models),
            i18ns: Arc::new(i18ns),
        })
    }

    fn issue_token(&self, credential: &str) -> Option<String> {
        self.token_generator
            .as_ref()
            .and_then(|generate| generate(credential))
            .filter(|token| !token.trim().is_empty())
    }

    fn auth_mode(&self) -> &'static str {
        if self.token_generator.is_some() {
            "custom"
        } else if self.token.is_some() {
            "static"
        } else {
            "none"
        }
    }
}

#[derive(Deserialize, afast::Tag)]
#[tag("Admin authorization header")]
struct AdminHeaders {
    authorization: Option<String>,
}

#[derive(Deserialize, Serialize)]
#[serde(transparent)]
struct AdminJson(JsonValue);

impl afast::Structure for AdminJson {
    fn structure() -> &'static afast::TagMeta {
        static META: afast::TagMeta = afast::TagMeta {
            name: "AdminJson",
            desc: "Dynamic afastorm admin response",
            kind: afast::TagKind::Struct(&[]),
        };
        &META
    }
}

pub fn service(url: &str) -> crate::Result<afast::Service> {
    let url = url.trim_matches('/');
    if url.is_empty()
        || url
            .chars()
            .any(|character| character.is_whitespace() || matches!(character, '?' | '#'))
    {
        return Err(crate::Error::custom(
            50001,
            "ORM admin URL must be a non-empty path without query or fragment",
        ));
    }
    let url = Box::leak(url.to_owned().into_boxed_str());
    let mut service = service!("afastorm_admin", "Afast ORM model administration" => {
        group("admin" => {
            get("/", admin_page),
            get("/api/models", list_models),
            post("/api/token", issue_token),
            get("/api/:model", list_rows),
            post("/api/:model", create_row),
            get("/api/:model/:id", get_row),
            patch("/api/:model/:id", update_row),
            delete("/api/:model/:id", delete_row),
        }),
    });
    let Some(group) = service.handlers.first_mut() else {
        return Err(crate::Error::custom(
            50001,
            "ORM admin service has no route group",
        ));
    };
    group.name = url;
    Ok(service)
}

#[derive(Deserialize, afast::Tag)]
#[tag("Admin model path")]
struct ModelPath {
    model: String,
}

#[derive(Deserialize, afast::Tag)]
#[tag("Admin row path")]
struct RowPath {
    model: String,
    id: String,
}

#[derive(Deserialize, afast::Tag)]
#[tag("Admin list query")]
struct ListQuery {
    page: Option<u64>,
    limit: Option<u64>,
    search: Option<String>,
}

#[derive(Deserialize, afast::Tag)]
#[tag("Admin page language")]
struct AdminPageQuery {
    lang: Option<String>,
}

#[derive(Deserialize, afast::Tag)]
#[tag("Admin optimistic lock version")]
struct VersionQuery {
    version: Option<String>,
}

#[derive(Deserialize, afast::Tag)]
#[tag("Admin login credential")]
struct AdminTokenRequest {
    credential: String,
}

#[post(desc("Issue an admin bearer token"))]
async fn issue_token(
    State(auth): State<AdminAuth>,
    Body(body): Body<AdminTokenRequest>,
) -> HttpResult<Json<AdminJson>> {
    let token = auth
        .issue_token(&body.credential)
        .ok_or_else(|| client_error(401, "invalid admin credential"))?;
    Ok(Json(AdminJson(json!({ "token": token }))))
}

#[get(desc("Generated Afast ORM administration page"))]
async fn admin_page(
    State(auth): State<AdminAuth>,
    Query(query): Query<AdminPageQuery>,
) -> HttpResult<Html> {
    let languages = auth
        .i18ns
        .iter()
        .map(|(code, translations)| json!({"code": code, "translations": translations}))
        .collect::<Vec<_>>();
    let requested_language = query.lang.as_deref().unwrap_or_default();
    let language = auth
        .i18ns
        .iter()
        .find(|(code, _)| code == requested_language)
        .map(|(code, _)| code.as_str())
        .or_else(|| auth.i18ns.first().map(|(code, _)| code.as_str()))
        .unwrap_or("zh-CN");
    let fallback: JsonValue = serde_json::from_str(
        r#"{
        "adminTitle": "管理后台",
        "topbarNote": "数据管理控制台",
        "language": "语言",
        "requestFailed": "请求失败",
        "retry": "重试",
        "credentialPrompt": "请输入登录凭据",
        "tokenPrompt": "请输入管理端访问令牌",
        "authSubtitle": "请输入凭据后继续访问管理端。",
        "credentialLabel": "访问凭据",
        "authContinue": "继续",
        "loginCancelled": "已取消登录",
        "authFailed": "认证失败",
        "credentialInvalid": "凭据无效，请重试",
        "authExpired": "认证已失效",
        "authRetry": "请重新登录",
        "dashboard": "工作台",
        "models": "数据模型",
        "noAdminModels": "没有启用管理端的模型",
        "chooseModel": "选择一个模型，查看和管理数据记录。",
        "fieldsCount": "{count} 个字段",
        "modelMissing": "模型不存在",
        "modelUnregistered": "地址中的模型未注册",
        "loading": "正在加载",
        "loadingRecords": "正在读取数据",
        "recordsSummary": "共 {total} 条记录 · 表 {table}",
        "edit": "编辑",
        "view": "查看",
        "delete": "删除",
        "create": "＋ 新建记录",
        "searchPlaceholder": "搜索记录…",
        "search": "搜索",
        "refresh": "刷新",
        "emptyRecords": "没有找到记录",
        "pageSummary": "第 {page} 页，共 {pages} 页",
        "previous": "上一页",
        "jumpTo": "跳转至",
        "page": "页",
        "jump": "跳转",
        "invalidPage": "页码无效",
        "pageRange": "请输入 1 到 {pages} 之间的页码",
        "dataLoadFailed": "数据加载失败",
        "loadFailed": "加载失败",
        "recordDetail": "记录详情",
        "editRecord": "编辑记录",
        "newRecord": "新建记录",
        "noEditableFields": "没有可编辑字段",
        "close": "关闭",
        "cancel": "取消",
        "save": "保存",
        "jsonPlaceholder": "请输入有效 JSON",
        "formatError": "格式错误",
        "validJson": "必须是有效 JSON",
        "validDateTime": "必须是有效日期时间",
        "createSuccess": "创建成功",
        "recordCreated": "记录已创建",
        "updateSuccess": "修改成功",
        "recordUpdated": "记录已更新",
        "createFailed": "创建失败",
        "updateFailed": "修改失败",
        "openRecordFailed": "打开记录失败",
        "confirmDelete": "确认删除",
        "irreversible": "此操作无法撤销",
        "deletePrefix": "确定删除记录 ",
        "deleteSuffix": " 吗？请再次确认后继续。",
        "confirmDeleteRecord": "确定删除记录 #{id} 吗？请再次确认后继续。",
        "confirm": "确认删除",
        "deleting": "正在删除…",
        "deleteSuccess": "删除成功",
        "recordDeleted": "记录已删除",
        "deleteFailed": "删除失败",
        "cannotLoadModels": "无法加载模型",
        "connectionFailed": "连接失败",
        "closeDialog": "关闭",
        "pageRangePrefix": "请输入 1 到 ",
        "pageRangeSuffix": " 之间的页码"
    }"#,
    )
    .map_err(admin_error)?;
    let config_json = json!({
        "authMode": auth.auth_mode(),
        "brandText": auth.brand_text.as_ref(),
        "i18ns": languages,
        "language": language,
        "fallback": fallback,
    })
    .to_string()
    .replace('<', "\\u003c")
    .replace('>', "\\u003e")
    .replace('&', "\\u0026");
    let html = r#"<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<meta name="color-scheme" content="light">
<title>Afast ORM</title>
<style>
:root{font:15px/1.5 Inter,-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif;color:#202536;background:#f4f6fb;font-synthesis:none;text-rendering:optimizeLegibility;--primary:#6258e8;--primary-dark:#5146d5;--border:#e7e9f1;--muted:#7d8498;--surface:#fff;--shadow:0 12px 36px #26315312}
*{box-sizing:border-box}body{margin:0;min-height:100vh}button,input,textarea,select{font:inherit}button{cursor:pointer}
.topbar{height:68px;background:#fff;border-bottom:1px solid var(--border);display:flex;align-items:center;padding:0 clamp(18px,4vw,56px);gap:18px;position:sticky;top:0;z-index:5}
.brand{font-weight:750;font-size:19px;letter-spacing:-.4px;color:#292e44;display:flex;align-items:center;gap:10px}.brand-mark{width:32px;height:32px;border-radius:10px;background:linear-gradient(135deg,#8278ff,#5749de);color:#fff;display:grid;place-items:center;font-size:16px}
.topbar-note{margin-left:auto;color:var(--muted);font-size:13px}.locale-picker{display:flex;align-items:center;gap:7px;color:var(--muted);font-size:13px}.locale-picker select{border:1px solid var(--border);border-radius:8px;background:#fff;padding:6px 9px;color:#41475b}.layout{max-width:1440px;margin:auto;padding:38px clamp(18px,4vw,56px) 60px}.crumb{display:flex;align-items:center;gap:9px;color:var(--muted);font-size:13px;margin:0 0 22px}.crumb button{border:0;background:none;color:var(--primary);padding:0;font-weight:600}.heading{display:flex;align-items:flex-end;justify-content:space-between;gap:20px;margin-bottom:24px}.heading h1{font-size:28px;letter-spacing:-.7px;margin:0 0 5px}.subtitle{color:var(--muted);margin:0}
.button{border:1px solid var(--border);background:#fff;color:#41475b;padding:9px 14px;border-radius:9px;font-weight:600;transition:transform .16s,box-shadow .16s,border-color .16s}.button:hover{transform:translateY(-1px);box-shadow:0 5px 14px #25315b12;border-color:#cdd0df}.button.primary{background:var(--primary);color:#fff;border-color:var(--primary)}.button.primary:hover{background:var(--primary-dark)}.button.danger{color:#d8465b}.button.small{font-size:13px;padding:6px 10px}
.model-grid{display:grid;grid-template-columns:repeat(auto-fill,minmax(230px,1fr));gap:16px}.model-card{border:1px solid var(--border);background:var(--surface);border-radius:14px;padding:22px;text-align:left;color:inherit;box-shadow:0 2px 6px #26315305;transition:transform .2s,box-shadow .2s,border-color .2s}.model-card:hover{transform:translateY(-3px);box-shadow:var(--shadow);border-color:#c9c5ff}.model-icon{width:42px;height:42px;border-radius:12px;background:#f0efff;color:var(--primary);display:grid;place-items:center;font-size:18px;margin-bottom:18px}.model-card strong{display:block;font-size:16px}.model-card span{display:block;color:var(--muted);font-size:13px;margin-top:5px}
.toolbar{display:flex;align-items:center;gap:10px;flex-wrap:wrap;background:#fff;border:1px solid var(--border);border-radius:13px;padding:14px;margin-bottom:16px}.search-wrap{position:relative;flex:1;min-width:220px}.search-wrap input{width:100%;padding:10px 12px 10px 36px;border:1px solid var(--border);border-radius:9px;outline:none}.search-wrap input:focus,.field input:focus,.field textarea:focus{border-color:#9991ff;box-shadow:0 0 0 3px #6258e81b}.search-icon{position:absolute;left:12px;top:9px;color:#9298aa}
.table-card{background:#fff;border:1px solid var(--border);border-radius:13px;overflow:hidden;box-shadow:0 2px 7px #26315305}table{width:100%;border-collapse:collapse}th,td{text-align:left;padding:13px 16px;border-bottom:1px solid #eef0f5}th{font-size:12px;text-transform:uppercase;letter-spacing:.5px;color:#81889a;background:#fafbfe;font-weight:700}td{color:#464d61}tbody tr:last-child td{border-bottom:0}tbody tr:hover{background:#fafaff}.actions{display:flex;gap:6px;white-space:nowrap}.empty{text-align:center;color:var(--muted);padding:48px 16px}
.pager{display:flex;align-items:center;justify-content:flex-end;gap:8px;margin-top:15px;color:var(--muted);font-size:13px}.pager input{width:72px;text-align:center;padding:7px;border:1px solid var(--border);border-radius:8px;outline:none}.pager .page-info{margin-right:auto}.pager button:disabled{opacity:.42;cursor:not-allowed;transform:none;box-shadow:none}
.overlay{position:fixed;inset:0;background:#171b2b70;backdrop-filter:blur(4px);display:grid;place-items:center;padding:20px;z-index:20;opacity:0;visibility:hidden;transition:opacity .2s,visibility .2s}.overlay.open{opacity:1;visibility:visible}.modal{width:min(620px,100%);max-height:min(86vh,900px);overflow:auto;background:#fff;border-radius:16px;box-shadow:0 24px 80px #11172d45;transform:translateY(14px) scale(.98);transition:transform .22s cubic-bezier(.2,.8,.2,1)}.overlay.open .modal{transform:translateY(0) scale(1)}.modal-head{display:flex;align-items:flex-start;justify-content:space-between;padding:22px 24px 16px;border-bottom:1px solid var(--border)}.modal-head h2{margin:0;font-size:20px}.modal-head p{margin:5px 0 0;color:var(--muted);font-size:13px}.icon-button{width:34px;height:34px;border:0;border-radius:9px;background:#f3f4f8;color:#656c7e;font-size:20px}.modal-body{padding:20px 24px}.modal-foot{padding:15px 24px 22px;display:flex;justify-content:flex-end;gap:9px}.form-grid{display:grid;grid-template-columns:1fr 1fr;gap:15px}.field{display:grid;gap:6px;color:#575e71;font-size:13px;font-weight:600}.field.full{grid-column:1/-1}.field input,.field textarea{width:100%;padding:10px 11px;border:1px solid var(--border);border-radius:8px;outline:none;color:#292e40;background:#fff}.field textarea{min-height:94px;resize:vertical}.field.checkbox{display:flex;align-items:center;gap:9px}.field.checkbox input{width:17px;height:17px;accent-color:var(--primary)}.field input:disabled,.field textarea:disabled{background:#f6f7fa;color:#747b8e}.confirm-mark{width:46px;height:46px;border-radius:50%;background:#fff1e9;color:#de782f;display:grid;place-items:center;font-size:22px;margin-bottom:12px}.confirm-text{color:#666d7e;margin:0}.confirm-name{font-weight:700;color:#343a4d}
#toasts{position:fixed;top:82px;right:22px;z-index:40;display:grid;gap:10px;width:min(380px,calc(100vw - 32px))}.toast{position:relative;display:grid;grid-template-columns:28px 1fr 24px;gap:10px;align-items:start;padding:14px 14px 17px;background:#fff;border:1px solid var(--border);border-left:4px solid #5b65d8;border-radius:11px;box-shadow:0 10px 32px #1e29451f;animation:toast-in .24s ease both;overflow:hidden}.toast.success{border-left-color:#25a879}.toast.error{border-left-color:#e45161}.toast.warning{border-left-color:#e9a23b}.toast.info{border-left-color:#5b65d8}.toast-icon{font-size:17px;font-weight:800}.toast.success .toast-icon{color:#25a879}.toast.error .toast-icon{color:#e45161}.toast.warning .toast-icon{color:#e9a23b}.toast.info .toast-icon{color:#5b65d8}.toast-title{font-weight:700;font-size:13px;margin-bottom:2px}.toast-message{color:#697084;font-size:13px;overflow-wrap:anywhere}.toast-close{border:0;background:none;color:#8a90a0;font-size:18px;line-height:1}.toast-progress{position:absolute;left:0;bottom:0;height:3px;background:currentColor;opacity:.55;animation:toast-progress 5s linear forwards}.toast.success .toast-progress{color:#25a879}.toast.error .toast-progress{color:#e45161}.toast.warning .toast-progress{color:#e9a23b}.toast.info .toast-progress{color:#5b65d8}@keyframes toast-in{from{opacity:0;transform:translateX(18px)}to{opacity:1;transform:translateX(0)}}@keyframes toast-out{to{opacity:0;transform:translateX(20px)}}@keyframes toast-progress{to{width:0}}
.loading{padding:60px;text-align:center;color:var(--muted)}.spinner{display:inline-block;width:20px;height:20px;border:2px solid #dedcfb;border-top-color:var(--primary);border-radius:50%;animation:spin .7s linear infinite;vertical-align:middle;margin-right:8px}@keyframes spin{to{transform:rotate(360deg)}}
@media(max-width:680px){.layout{padding-top:26px}.heading{align-items:flex-start;flex-direction:column}.heading .button{width:100%}.toolbar{align-items:stretch}.search-wrap{flex-basis:100%}.table-card{overflow-x:auto}table{min-width:620px}.form-grid{grid-template-columns:1fr}.field.full{grid-column:auto}.pager{justify-content:center;flex-wrap:wrap}.pager .page-info{flex-basis:100%;text-align:center}.modal-head,.modal-body{padding-left:18px;padding-right:18px}.modal-foot{padding-left:18px;padding-right:18px}}
</style>
</head>
<body>
<header class="topbar"><div class="brand"><span class="brand-mark">__ADMIN_LOGO__</span><span id="brandText">__ADMIN_BRAND__</span></div><div class="topbar-note" id="topbarNote"></div><label class="locale-picker"><span id="languageLabel"></span><select id="localeSelector"></select></label></header>
<main class="layout"><div id="crumb" class="crumb"></div><section id="content"><div class="loading"><span class="spinner"></span>正在加载</div></section></main>
<div id="modalRoot"></div><div id="toasts" aria-live="polite"></div>
<script>
const adminConfig=__ADMIN_CONFIG__;
const esc=value=>String(value&&typeof value==='object'?JSON.stringify(value):value??'').replace(/[&<>"']/g,char=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[char]));
const root=location.pathname.replace(/\/$/,'');
const i18ns=adminConfig.i18ns;
const defaultI18n=i18ns[0];
const languageParams=new URLSearchParams(location.search);
let localeCode=i18ns.some(item=>item.code===languageParams.get('lang'))?languageParams.get('lang'):defaultI18n.code;
const activeI18n=i18ns.find(item=>item.code===localeCode)||defaultI18n;
let token=adminConfig.authMode==='custom'?localStorage.getItem('afast-orm-admin-token'):sessionStorage.getItem('afast-orm-admin-token');
const $=selector=>document.querySelector(selector);
function notify(type,title,message){const icons={success:'✓',error:'!',warning:'⚠',info:'i'};const toast=document.createElement('div');toast.className='toast '+type;toast.innerHTML='<span class="toast-icon">'+icons[type]+'</span><div><div class="toast-title">'+esc(title)+'</div><div class="toast-message">'+esc(message)+'</div></div><button class="toast-close" aria-label="关闭">×</button><span class="toast-progress"></span>';$('#toasts').append(toast);let remaining=5000,started=Date.now(),timer;const close=()=>{clearTimeout(timer);toast.style.animation='toast-out .18s ease forwards';setTimeout(()=>toast.remove(),180)};const resume=()=>{started=Date.now();toast.querySelector('.toast-progress').style.animation='toast-progress '+remaining+'ms linear forwards';timer=setTimeout(close,remaining)};toast.addEventListener('mouseenter',()=>{clearTimeout(timer);remaining=Math.max(0,remaining-(Date.now()-started));toast.querySelector('.toast-progress').style.animationPlayState='paused'});toast.addEventListener('mouseleave',resume);toast.querySelector('.toast-close').onclick=close;resume()}
function route(){const raw=location.hash.slice(1);if(!raw)return{model:null,page:1,search:''};const [name,query='']=raw.split('?');const params=new URLSearchParams(query);return{model:decodeURIComponent(name.startsWith('/')?name.slice(1):name),page:Math.max(1,Number(params.get('page'))||1),search:params.get('search')||''}}
function setRoute(model,page=1,search='',push=false){const hash=model?'#/'+encodeURIComponent(model)+'?page='+page+'&search='+encodeURIComponent(search):'';if(push)location.hash=hash;else history.replaceState(null,'',location.pathname+location.search+hash);render()}
let models=[];
function renderHome(){ $('#crumb').innerHTML='<span>工作台</span>';$('#content').innerHTML='<div class="heading"><div><h1>数据模型</h1><p class="subtitle">选择一个模型，查看和管理数据记录。</p></div></div><div class="model-grid">'+models.map((model,index)=>'<button class="model-card" data-open="'+esc(model.name)+'"><span class="model-icon">'+(index%2?'◇':'▦')+'</span><strong>'+esc(model.label)+'</strong><span>'+esc(model.name)+' · '+model.fields.length+' 个字段</span></button>').join('')+'</div>';document.querySelectorAll('[data-open]').forEach(button=>button.onclick=()=>setRoute(button.dataset.open))}
async function renderModel(model,page,search){$('#crumb').innerHTML='<button id="homeLink">工作台</button><span>›</span><span>'+esc(model.label)+'</span>';$('#homeLink').onclick=()=>{location.hash=''};$('#content').innerHTML='<div class="loading"><span class="spinner"></span>正在读取数据</div>';try{const data=await api(encodeURIComponent(model.name)+'?page='+page+'&limit=20&search='+encodeURIComponent(search));const fields=model.fields.filter(field=>field.list).sort((a,b)=>(a.order??Number.MAX_SAFE_INTEGER)-(b.order??Number.MAX_SAFE_INTEGER)),pages=Math.max(1,Math.ceil(data.total/data.limit));const rows=data.items.map(row=>'<tr>'+fields.map(field=>'<td>'+esc(row[field.name])+'</td>').join('')+'<td><div class="actions">'+((model.detail||model.update)?'<button class="button small" data-edit="'+esc(row.__id)+'">'+(model.update?'编辑':'查看')+'</button>':'')+(model.delete?'<button class="button small danger" data-delete="'+esc(row.__id)+'" data-version="'+esc(row.__version)+'">删除</button>':'')+'</div></td></tr>').join('');$('#content').innerHTML='<div class="heading"><div><h1>'+esc(model.label)+'</h1><p class="subtitle">共 '+data.total+' 条记录 · 表 <code>'+esc(model.name)+'</code></p></div>'+(model.create?'<button class="button primary" id="createBtn">＋ 新建记录</button>':'')+'</div><div class="toolbar"><div class="search-wrap"><span class="search-icon">⌕</span><input id="searchInput" type="search" placeholder="搜索记录…" value="'+esc(search)+'"></div><button class="button" id="searchBtn">搜索</button><button class="button" id="refreshBtn">刷新</button></div><div class="table-card">'+(rows?'<table><thead><tr>'+fields.map(field=>'<th>'+esc(field.label)+'</th>').join('')+'<th>操作</th></tr></thead><tbody>'+rows+'</tbody></table>':'<div class="empty">没有找到记录</div>')+'</div><div class="pager"><span class="page-info">第 '+data.page+' 页，共 '+pages+' 页</span><button class="button small" id="prevBtn" '+(page<=1?'disabled':'')+'>上一页</button><span>跳转至</span><input id="pageInput" type="number" min="1" max="'+pages+'" value="'+page+'" aria-label="页码"><span>页</span><button class="button small" id="jumpBtn">跳转</button><button class="button small" id="nextBtn" '+(page>=pages?'disabled':'')+'>下一页</button></div>';$('#searchBtn').onclick=()=>setRoute(model.name,1,$('#searchInput').value.trim());$('#searchInput').onkeydown=event=>{if(event.key==='Enter')$('#searchBtn').click()};$('#refreshBtn').onclick=()=>renderModel(model,page,search);$('#prevBtn').onclick=()=>setRoute(model,page-1,search);$('#nextBtn').onclick=()=>setRoute(model,page+1,search);$('#jumpBtn').onclick=()=>{const target=Number($('#pageInput').value);if(!Number.isInteger(target)||target<1||target>pages){notify('warning','页码无效','请输入 1 到 '+pages+' 之间的页码');return}setRoute(model,target,search)};$('#pageInput').onkeydown=event=>{if(event.key==='Enter')$('#jumpBtn').click()};if(model.create)$('#createBtn').onclick=()=>openEditor(model);document.querySelectorAll('[data-edit]').forEach(button=>button.onclick=()=>openEditor(model,button.dataset.edit));document.querySelectorAll('[data-delete]').forEach(button=>button.onclick=()=>confirmDelete(model,button.dataset.delete,button.dataset.version,page,search));}catch(error){$('#content').innerHTML='<div class="empty">数据加载失败</div>';notify('error','加载失败',error.message)}}
async function render(){if(!models.length){$('#crumb').textContent=t('dashboard');$('#content').innerHTML='<div class="empty">'+esc(t('noAdminModels'))+'</div>';return}const state=route();if(!state.model){renderHome();return}const model=models.find(item=>item.name===state.model);if(!model){notify('warning','模型不存在','地址中的模型未注册');history.replaceState(null,'',location.pathname+location.search);renderHome();return}await renderModel(model,state.page,state.search)}
function modal(title,subtitle,body,buttons){$('#modalRoot').innerHTML='<div class="overlay" id="overlay"><section class="modal" role="dialog" aria-modal="true"><div class="modal-head"><div><h2>'+esc(title)+'</h2><p>'+esc(subtitle)+'</p></div><button class="icon-button" id="modalClose" aria-label="关闭">×</button></div><div class="modal-body">'+body+'</div><div class="modal-foot">'+buttons+'</div></section></div>';const overlay=$('#overlay');requestAnimationFrame(()=>overlay.classList.add('open'));$('#modalClose').onclick=closeModal;document.onkeydown=event=>{if(event.key==='Escape')closeModal()}}
function closeModal(){const overlay=$('#overlay');if(!overlay)return;overlay.classList.remove('open');document.onkeydown=null;setTimeout(()=>$('#modalRoot').replaceChildren(),220)}
function fieldTypeLabel(type){return({bool:'BOOLEAN',i16:'SMALLINT',i32:'INTEGER',i64:'BIGINT',f32:'REAL',f64:'DOUBLE',string:'TEXT',bytes:'BINARY',json:'JSON',uuid:'UUID',date:'DATE',time:'TIME',timestamp:'DATETIME',timestamptz:'DATETIME WITH TIME ZONE',numeric:'NUMERIC',bignumeric:'BIG NUMERIC'})[type]||type.toUpperCase()}
function editorValue(field,value){if(value==null)return'';if(field.type==='json')return JSON.stringify(value,null,2);const raw=String(value);if(field.type==='timestamp')return raw.replace(' ','T').slice(0,19);if(field.type==='timestamptz'){const parsed=new Date(raw.replace(' ','T').replace(/([+-]\d{2})$/,'$1:00'));if(!Number.isNaN(parsed.getTime()))return new Date(parsed.getTime()-parsed.getTimezoneOffset()*60000).toISOString().slice(0,19)}return raw}
async function openEditor(model,id){
 try{
  const existing=id?await api(encodeURIComponent(model.name)+'/'+encodeURIComponent(id)):{};
  const viewOnly=!!id&&!model.update;
  const fields=model.fields.filter(field=>id?(model.update?field.update:field.detail):field.create);
  const title=field=>'<span class="field-label">'+esc(field.label)+' <small style="font-size:11px;font-weight:500;color:#8990a2">'+esc(fieldTypeLabel(field.type))+'</small></span>';
  const inputs=fields.map(field=>{
   const value=existing[field.name];
   const nullable=field.nullable;
   const isNull=nullable&&value==null;
   const disabled=field.readonly||viewOnly||isNull;
   const name=esc(field.name);
   const nullToggle=nullable?'<label class="null-toggle" style="display:flex;align-items:center;gap:6px;font-size:12px;color:#7d8498"><input type="checkbox" name="__null_'+name+'" '+(isNull?'checked ':'')+(field.readonly||viewOnly?'disabled':'')+'>NULL</label>':'';
   const common='name="'+name+'" '+(disabled?'disabled':'');
   let control;
   if(field.type==='bool')control='<label class="field checkbox"><input type="checkbox" '+common+(value?' checked':'')+'>'+title(field)+'</label>';
   else if(field.type==='json')control='<label class="field full">'+title(field)+'<textarea '+common+' placeholder="请输入有效 JSON">'+esc(editorValue(field,value))+'</textarea></label>';
   else{
    const numeric=['i16','i32','i64','f32','f64'].includes(field.type);
    const type=({date:'date',time:'time',timestamp:'datetime-local',timestamptz:'datetime-local'})[field.type]||(numeric?'number':'text');
    const step=['time','timestamp','timestamptz'].includes(field.type)?' step="1"':'';
    control='<label class="field">'+title(field)+'<input type="'+type+'" '+common+step+' value="'+esc(editorValue(field,value))+'"></label>'
   }
   return nullable?'<div class="nullable-field">'+nullToggle+control+'</div>':control
  }).join('');
  modal(id?(viewOnly?'记录详情':'编辑记录'):'新建记录',model.label+' · '+model.name,'<form id="recordForm"><div class="form-grid">'+(inputs||'<p class="confirm-text">没有可编辑字段</p>')+'</div></form>',(viewOnly?'<button class="button" id="modalCancel">关闭</button>':'<button class="button" id="modalCancel">取消</button><button class="button primary" id="saveBtn">保存</button>'));
  $('#modalCancel').onclick=closeModal;
  if(viewOnly)return;
  const form=$('#recordForm');
  for(const field of fields){
   if(!field.nullable)continue;
   const nullInput=form.elements['__null_'+field.name];
   const input=form.elements[field.name];
   nullInput.onchange=()=>{input.disabled=field.readonly||nullInput.checked};
  }
  $('#saveBtn').onclick=async()=>{
   const body={};
   for(const field of fields){
    const input=form.elements[field.name];
    if(!input)continue;
    if(field.nullable&&form.elements['__null_'+field.name].checked){body[field.name]=null;continue}
    if(field.type==='bool')body[field.name]=input.checked;
    else if(['i16','i32','i64','f32','f64'].includes(field.type)){
     if(input.value===''){notify('warning','格式错误',field.label+' 必须填写');input.focus();return}
     body[field.name]=Number(input.value)
    }else if(field.type==='json'){
     try{body[field.name]=JSON.parse(input.value)}catch{notify('warning','格式错误',field.label+' 必须是有效 JSON');input.focus();return}
    }else if(field.type==='timestamptz'){
     const date=new Date(input.value);
     if(Number.isNaN(date.getTime())){notify('warning','格式错误',field.label+' 必须是有效日期时间');input.focus();return}
     body[field.name]=date.toISOString()
    }else if(field.type==='timestamp'){
     if(!input.value){notify('warning','格式错误',field.label+' 必须是有效日期时间');input.focus();return}
     body[field.name]=input.value.replace('T',' ')
    }else body[field.name]=input.value
   }
   if(id&&existing.version!==undefined)body.version=existing.version;
   try{
    const response=await api(encodeURIComponent(model.name)+(id?'/'+encodeURIComponent(id):''),{method:id?'PATCH':'POST',headers:{'content-type':'application/json'},body:JSON.stringify(body)});
    if(!response.ok)throw new Error('数据未发生变更');
    closeModal();notify('success',id?'修改成功':'创建成功',id?'记录已更新':'记录已创建');await render()
   }catch(error){notify('error',id?'修改失败':'创建失败',error.message)}
  }
 }catch(error){notify('error','打开记录失败',error.message)}
}
function confirmDelete(model,id,version,page,search){modal('确认删除','此操作无法撤销','<div class="confirm-mark">!</div><p class="confirm-text">确定删除记录 <span class="confirm-name">#'+esc(id)+'</span> 吗？请再次确认后继续。</p>','<button class="button" id="deleteCancel">取消</button><button class="button danger" id="deleteConfirm">确认删除</button>');$('#deleteCancel').onclick=closeModal;$('#deleteConfirm').onclick=async()=>{const button=$('#deleteConfirm');button.disabled=true;button.textContent='正在删除…';try{const response=await api(encodeURIComponent(model.name)+'/'+encodeURIComponent(id)+'?version='+encodeURIComponent(version??''),{method:'DELETE'});if(!response.ok)throw new Error('记录不存在或未发生变更');closeModal();notify('success','删除成功','记录已删除');const state=route();await renderModel(model,page,search)}catch(error){button.disabled=false;button.textContent='确认删除';notify('error','删除失败',error.message)}}}
window.addEventListener('hashchange',render);
function t(key){return activeI18n.translations[key]??defaultI18n.translations[key]??adminConfig.fallback[key]??key}
const sourceKeys={'工作台':'dashboard','数据模型':'models','选择一个模型，查看和管理数据记录。':'chooseModel','模型不存在':'modelMissing','地址中的模型未注册':'modelUnregistered','正在加载':'loading','正在读取数据':'loadingRecords','编辑':'edit','查看':'view','删除':'delete','＋ 新建记录':'create','搜索记录…':'searchPlaceholder','搜索':'search','刷新':'refresh','没有找到记录':'emptyRecords','上一页':'previous','跳转至':'jumpTo','页':'page','跳转':'jump','页码无效':'invalidPage','数据加载失败':'dataLoadFailed','加载失败':'loadFailed','记录详情':'recordDetail','编辑记录':'editRecord','新建记录':'newRecord','没有可编辑字段':'noEditableFields','关闭':'close','取消':'cancel','保存':'save','请输入有效 JSON':'jsonPlaceholder','格式错误':'formatError','创建成功':'createSuccess','记录已创建':'recordCreated','修改成功':'updateSuccess','记录已更新':'recordUpdated','创建失败':'createFailed','修改失败':'updateFailed','打开记录失败':'openRecordFailed','确认删除':'confirmDelete','此操作无法撤销':'irreversible','确定删除记录 ':'deletePrefix',' 吗？请再次确认后继续。':'deleteSuffix','正在删除…':'deleting','删除成功':'deleteSuccess','记录已删除':'recordDeleted','删除失败':'deleteFailed','无法加载模型':'cannotLoadModels','连接失败':'connectionFailed','请输入 1 到 ':'pageRangePrefix',' 之间的页码':'pageRangeSuffix'};
function localizeTextNode(node){
 const original=node.nodeValue;
 const trimmed=original.trim();
 const key=sourceKeys[trimmed];
 let translated;
 if(key)translated=t(key);
 else if((translated=trimmed.match(/^共 (\d+) 条记录 · 表 ?$/))){translated=t('recordsSummary').replace('{total}',translated[1]).replace('{table}','').trimEnd()+' '}
 else if((translated=trimmed.match(/^(.+) · (\d+) 个字段$/))){translated=translated[1]+' · '+t('fieldsCount').replace('{count}',translated[2])}
 else if((translated=trimmed.match(/^第 (\d+) 页，共 (\d+) 页$/))){translated=t('pageSummary').replace('{page}',translated[1]).replace('{pages}',translated[2])}
 else if((translated=trimmed.match(/^请输入 1 到 (\d+) 之间的页码$/))){translated=t('pageRange').replace('{pages}',translated[1])}
 else if(trimmed.endsWith(' 必须是有效 JSON')){translated=trimmed.slice(0,-' 必须是有效 JSON'.length)+t('validJson')}
 else if(trimmed.endsWith(' 必须是有效日期时间')){translated=trimmed.slice(0,-' 必须是有效日期时间'.length)+t('validDateTime')}
 if(translated===null)return;
 if(translated!==undefined){
  const start=original.indexOf(trimmed);
  const localized=original.slice(0,start)+translated+original.slice(start+trimmed.length);
  if(localized!==original)node.nodeValue=localized
 }
}
function localizeSubtree(rootNode){const walker=document.createTreeWalker(rootNode,NodeFilter.SHOW_TEXT);while(walker.nextNode())localizeTextNode(walker.currentNode);if(rootNode.nodeType===Node.ELEMENT_NODE){for(const element of [rootNode,...rootNode.querySelectorAll('[placeholder],[aria-label]')])for(const attribute of ['placeholder','aria-label']){const value=element.getAttribute(attribute);if(value&&sourceKeys[value])element.setAttribute(attribute,t(sourceKeys[value]))}}}
function applyLocale(){document.documentElement.lang=localeCode;document.title=adminConfig.brandText+' - '+t('adminTitle');$('#topbarNote').textContent=t('topbarNote');$('#languageLabel').textContent=t('language');const select=$('#localeSelector');select.innerHTML=i18ns.map(item=>'<option value="'+esc(item.code)+'">'+esc(item.code)+'</option>').join('');select.value=localeCode;select.onchange=()=>{const url=new URL(location.href);url.searchParams.set('lang',select.value);location.assign(url)};localizeSubtree(document.body);new MutationObserver(records=>records.forEach(record=>record.addedNodes.forEach(node=>{if(node.nodeType===Node.TEXT_NODE)localizeTextNode(node);else if(node.nodeType===Node.ELEMENT_NODE)localizeSubtree(node)}))).observe(document.body,{childList:true,subtree:true})}
function persistToken(value){token=value;const storage=adminConfig.authMode==='custom'?localStorage:sessionStorage;if(value)storage.setItem('afast-orm-admin-token',value);else storage.removeItem('afast-orm-admin-token')}
function requestCredential(){
 return new Promise((resolve,reject)=>{
  let settled=false;
  const cancel=()=>{if(settled)return;settled=true;closeModal();reject(new Error(t('loginCancelled')))};
  modal(adminConfig.authMode==='custom'?t('credentialPrompt'):t('tokenPrompt'),t('authSubtitle'),'<form id="adminCredentialForm"><label class="field">'+esc(t('credentialLabel'))+'<input id="adminCredential" type="password" autocomplete="current-password" required></label></form>','<button class="button" id="authCancel">'+esc(t('cancel'))+'</button><button class="button primary" type="submit" form="adminCredentialForm">'+esc(t('authContinue'))+'</button>');
  $('#modalClose').onclick=cancel;
  document.onkeydown=event=>{if(event.key==='Escape')cancel()};
  $('#adminCredentialForm').onsubmit=event=>{
   event.preventDefault();
   if(settled)return;
   const credential=$('#adminCredential').value.trim();
   if(!credential){notify('warning',t('authFailed'),t('credentialInvalid'));$('#adminCredential').focus();return}
   settled=true;
   closeModal();
   resolve(credential);
  };
  $('#authCancel').onclick=cancel;
  requestAnimationFrame(()=>$('#adminCredential')?.focus());
 });
}
async function api(path,options={}){
 if(adminConfig.authMode==='none'){
  const response=await fetch(root+'/api/'+path,options);
  if(!response.ok){const message=await response.text();throw new Error(message||t('requestFailed')+' (HTTP '+response.status+')')}
  return response.json()
 }
 if(!token){
  const credential=await requestCredential();
  if(adminConfig.authMode==='custom'){
   const login=await fetch(root+'/api/token',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify({credential})});
   if(!login.ok){notify('warning',t('authFailed'),t('credentialInvalid'));throw new Error(t('credentialInvalid'))}
   const result=await login.json();
   if(typeof result.token!=='string'||!result.token.trim())throw new Error(t('credentialInvalid'));
   persistToken(result.token)
  }else persistToken(credential.trim())
 }
 const headers={...options.headers,authorization:'Bearer '+token};
 const response=await fetch(root+'/api/'+path,{...options,headers});
 if(response.status===401){persistToken(null);notify('warning',t('authExpired'),t('authRetry'));throw new Error(t('authExpired')+' - '+t('authRetry'))}
 if(!response.ok){const message=await response.text();throw new Error(message||t('requestFailed')+' (HTTP '+response.status+')')}
 return response.json()
}
applyLocale();
api('models?lang='+encodeURIComponent(localeCode)).then(result=>{models=result;render()}).catch(error=>{ $('#content').innerHTML='<div class="empty">无法加载模型<br><button class="button" id="retryLoad">'+esc(t('retry'))+'</button></div>';$('#retryLoad').onclick=()=>location.reload();notify('error','连接失败',error.message)});
</script>
</body></html>"#
        .replace("__ADMIN_LOGO__", &escape_html(&auth.logo_letter))
        .replace("__ADMIN_BRAND__", &escape_html(&auth.brand_text))
        .replace("__ADMIN_CONFIG__", &config_json);
    Ok(Html(html))
}

fn authorize(auth: &AdminAuth, headers: &AdminHeaders) -> HttpResult<()> {
    if let Some(verify) = &auth.token_verifier {
        let supplied = headers
            .authorization
            .as_deref()
            .and_then(|value| value.strip_prefix("Bearer "))
            .unwrap_or_default();
        return if verify(supplied) {
            Ok(())
        } else {
            Err(client_error(401, "invalid admin bearer token"))
        };
    }
    let Some(token) = auth.token.as_deref() else {
        return Ok(());
    };
    let supplied = headers
        .authorization
        .as_deref()
        .and_then(|value| value.strip_prefix("Bearer "))
        .unwrap_or_default()
        .as_bytes();
    let expected = token.as_bytes();
    let mut difference = expected.len() ^ supplied.len();
    for index in 0..expected.len().max(supplied.len()) {
        difference |= usize::from(
            expected.get(index).copied().unwrap_or_default()
                ^ supplied.get(index).copied().unwrap_or_default(),
        );
    }
    if difference == 0 {
        Ok(())
    } else {
        Err(client_error(401, "invalid admin bearer token"))
    }
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[get(desc("List registered admin models"))]
async fn list_models(
    State(auth): State<AdminAuth>,
    Header(header): Header<AdminHeaders>,
    Query(query): Query<AdminPageQuery>,
) -> HttpResult<Json<AdminJson>> {
    authorize(auth, &header)?;
    let language = query.lang.as_deref().unwrap_or_else(|| {
        auth.i18ns
            .first()
            .map(|(code, _)| code.as_str())
            .unwrap_or("zh-CN")
    });
    let models = auth
        .models
        .iter()
        .map(|model| {
            let info = model.database;
            let metadata = model.admin;
            let fields = model
                    .fields
                    .iter()
                    .filter(|field| {
                        field_is_admin_visible(field)
                            || Some(field.column_name) == info.version_field
                    })
                    .collect::<Vec<_>>();
            json!({
                    "name": info.table_name,
                    "label": model.label(language, &auth.i18ns),
                    "list": metadata.list,
                    "detail": metadata.detail,
                    "create": metadata.create,
                    "update": metadata.update,
                    "delete": metadata.delete,
                    "fields": fields.iter().map(|field| field_json(field, language, &auth.i18ns)).collect::<Vec<_>>(),
                })
        })
        .collect::<Vec<_>>();
    Ok(Json(AdminJson(JsonValue::Array(models))))
}

#[get(desc("List rows for an admin model"))]
async fn list_rows(
    State(orm): State<Orm>,
    State(auth): State<AdminAuth>,
    Header(header): Header<AdminHeaders>,
    Param(path): Param<ModelPath>,
    Query(query): Query<ListQuery>,
) -> HttpResult<Json<AdminJson>> {
    authorize(auth, &header)?;
    let info = admin_model(auth, &path.model, "list")?;
    let mut fields = info
        .fields
        .iter()
        .filter(|field| field.admin.list && !field.admin.hidden)
        .collect::<Vec<_>>();
    fields.sort_by_key(|field| field.admin.order.unwrap_or(u32::MAX));
    if fields.is_empty() {
        return Err(client_error(400, "model has no list-visible fields"));
    }
    let mut selected_columns = fields
        .iter()
        .map(|field| select_column(orm, field))
        .collect::<afast::Result<Vec<_>>>()?;
    let pk = primary_key(info)?;
    if !fields
        .iter()
        .any(|field| field.column_name == pk.column_name)
    {
        selected_columns.push(quote_column(orm, pk.column_name)?);
    }
    if let Some(version) = info.version_field
        && !fields.iter().any(|field| field.column_name == version)
    {
        selected_columns.push(quote_column(orm, version)?);
    }
    let table = quote_table(orm, info)?;
    let mut sql = format!("SELECT {} FROM {table}", selected_columns.join(", "));
    let mut binds = Vec::new();
    let mut predicates = Vec::new();
    if let Some(deleted_at) = info.soft_delete_field {
        predicates.push(format!("{} IS NULL", quote_column(orm, deleted_at)?));
    }
    if let Some(search) = query.search.filter(|search| !search.is_empty()) {
        let search_fields = fields
            .iter()
            .filter(|field| field.admin.search && field.field_type == FieldType::String)
            .collect::<Vec<_>>();
        if !search_fields.is_empty() {
            let mut search_predicates = Vec::with_capacity(search_fields.len());
            for field in search_fields {
                binds.push(Value::String(format!("%{search}%")));
                search_predicates.push(format!("{} LIKE ?", quote_column(orm, field.column_name)?));
            }
            predicates.push(format!("({})", search_predicates.join(" OR ")));
        }
    }
    if !predicates.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&predicates.join(" AND "));
    }
    let mut count_sql = format!("SELECT COUNT(*) FROM {table}");
    if !predicates.is_empty() {
        count_sql.push_str(" WHERE ");
        count_sql.push_str(&predicates.join(" AND "));
    }
    let (total,): (i64,) = orm
        .fetch_one(
            &Statement::from_sql(orm.dialect(), &count_sql, binds.clone()).map_err(admin_error)?,
        )
        .await
        .map_err(admin_error)?;
    let limit = query.limit.unwrap_or(50).clamp(1, 100);
    let page = query.page.unwrap_or(1).max(1);
    let offset = page.saturating_sub(1).saturating_mul(limit);
    sql.push_str(&format!(" LIMIT {limit} OFFSET {offset}"));
    let rows = orm
        .fetch_rows(&Statement::from_sql(orm.dialect(), &sql, binds).map_err(admin_error)?)
        .await
        .map_err(admin_error)?;
    let items = rows
        .iter()
        .map(|row| {
            let mut item = row_to_json(row, &fields).map_err(admin_error)?;
            let id = row_id(row, pk)?;
            item.as_object_mut()
                .expect("row serialization creates an object")
                .insert("__id".to_owned(), JsonValue::String(id));
            if let Some(version) = info.version_field {
                let version = <i64 as OrmDecode>::decode(row, version).map_err(admin_error)?;
                item.as_object_mut()
                    .expect("row serialization creates an object")
                    .insert("__version".to_owned(), JsonValue::from(version));
            }
            Ok::<JsonValue, afast::Error>(item)
        })
        .collect::<afast::Result<Vec<_>>>()?;
    Ok(Json(AdminJson(
        json!({ "items": items, "page": page, "limit": limit, "total": total }),
    )))
}

#[get(desc("Get an admin model row"))]
async fn get_row(
    State(orm): State<Orm>,
    State(auth): State<AdminAuth>,
    Header(header): Header<AdminHeaders>,
    Param(path): Param<RowPath>,
) -> HttpResult<Json<AdminJson>> {
    authorize(auth, &header)?;
    let info = admin_model(auth, &path.model, "detail")?;
    let fields = info
        .fields
        .iter()
        .filter(|field| (field.admin.detail || field.admin.update) && !field.admin.hidden)
        .collect::<Vec<_>>();
    if fields.is_empty() {
        return Err(client_error(400, "model has no detail-visible fields"));
    }
    let statement = row_select(orm, info, &fields, &path.id, true)?;
    let row = orm
        .fetch_rows(&statement)
        .await
        .map_err(admin_error)?
        .into_iter()
        .next()
        .ok_or_else(|| client_error(404, "row not found"))?;
    let mut value = row_to_json(&row, &fields).map_err(admin_error)?;
    if let Some(version) = info.version_field {
        let version = <i64 as OrmDecode>::decode(&row, version).map_err(admin_error)?;
        value
            .as_object_mut()
            .expect("row serialization creates an object")
            .insert("version".to_owned(), JsonValue::from(version));
    }
    Ok(Json(AdminJson(value)))
}

#[post(desc("Create an admin model row"))]
async fn create_row(
    State(orm): State<Orm>,
    State(auth): State<AdminAuth>,
    Header(header): Header<AdminHeaders>,
    Param(path): Param<ModelPath>,
    Body(body): Body<AdminJson>,
) -> HttpResult<Json<AdminJson>> {
    authorize(auth, &header)?;
    let info = admin_model(auth, &path.model, "create")?;
    let object = body
        .0
        .as_object()
        .ok_or_else(|| client_error(400, "request body must be an object"))?;
    let mut fields = Vec::new();
    let mut values = Vec::new();
    let mut expressions = Vec::new();
    for field in &info.fields {
        if field.auto_create || field.auto_update {
            fields.push(quote_column(orm, field.column_name)?);
            expressions.push(orm.dialect().current_value(field.field_type).to_owned());
            continue;
        }
        if field.admin.create
            && !field.admin.readonly
            && !field.admin.hidden
            && !(field.primary_key && field.auto_increment)
            && !field.soft_delete
            && !field.version
            && let Some(value) = object.get(field.column_name)
        {
            fields.push(quote_column(orm, field.column_name)?);
            values.push(json_to_value(value, field)?);
            expressions.push(field_value_placeholder(orm, field));
        }
    }
    if let Some(version) = info.version_field {
        fields.push(quote_column(orm, version)?);
        values.push(Value::I64(1));
        expressions.push("?".to_owned());
    }
    if fields.is_empty() {
        return Err(client_error(400, "no writable fields were provided"));
    }
    let sql = format!(
        "INSERT INTO {} ({}) VALUES ({})",
        quote_table(orm, info)?,
        fields.join(", "),
        expressions.join(", ")
    );
    orm.execute(&Statement::from_sql(orm.dialect(), &sql, values).map_err(admin_error)?)
        .await
        .map_err(admin_error)?;
    Ok(Json(AdminJson(json!({ "ok": true }))))
}

#[patch(desc("Update an admin model row"))]
async fn update_row(
    State(orm): State<Orm>,
    State(auth): State<AdminAuth>,
    Header(header): Header<AdminHeaders>,
    Param(path): Param<RowPath>,
    Body(body): Body<AdminJson>,
) -> HttpResult<Json<AdminJson>> {
    authorize(auth, &header)?;
    let info = admin_model(auth, &path.model, "update")?;
    let object = body
        .0
        .as_object()
        .ok_or_else(|| client_error(400, "request body must be an object"))?;
    let primary = primary_key(info)?;
    let mut assignments = Vec::new();
    let mut binds = Vec::new();
    for field in &info.fields {
        if field.auto_update {
            assignments.push(format!(
                "{} = {}",
                quote_column(orm, field.column_name)?,
                orm.dialect().current_value(field.field_type)
            ));
            continue;
        }
        if field.auto_create
            || !field.admin.update
            || field.admin.readonly
            || field.admin.hidden
            || field.primary_key
            || field.soft_delete
            || field.version
        {
            continue;
        }
        if let Some(value) = object.get(field.column_name) {
            binds.push(json_to_value(value, field)?);
            assignments.push(format!(
                "{} = {}",
                quote_column(orm, field.column_name)?,
                field_value_placeholder(orm, field)
            ));
        }
    }
    if assignments.is_empty() {
        return Err(client_error(400, "no writable fields were provided"));
    }
    if let Some(version) = info.version_field {
        let version = quote_column(orm, version)?;
        assignments.push(format!("{version} = {version} + 1"));
    }
    let id = parse_id(&path.id, primary)?;
    binds.push(id);
    let mut where_clause = format!("{} = {}", quote_column(orm, primary.column_name)?, "?");
    if let Some(soft_delete) = info.soft_delete_field {
        where_clause.push_str(&format!(" AND {} IS NULL", quote_column(orm, soft_delete)?));
    }
    if let Some(version_column) = info.version_field {
        let expected = object
            .get(version_column)
            .and_then(JsonValue::as_i64)
            .ok_or_else(|| client_error(400, "version is required for a versioned update"))?;
        binds.push(Value::I64(expected));
        where_clause.push_str(&format!(
            " AND {} = {}",
            quote_column(orm, version_column)?,
            "?"
        ));
    }
    let sql = format!(
        "UPDATE {} SET {} WHERE {where_clause}",
        quote_table(orm, info)?,
        assignments.join(", ")
    );
    let result = orm
        .execute(&Statement::from_sql(orm.dialect(), &sql, binds).map_err(admin_error)?)
        .await
        .map_err(admin_error)?;
    if result.rows_affected == 0 && info.version_field.is_some() {
        return Err(client_error(409, "optimistic lock conflict"));
    }
    Ok(Json(AdminJson(json!({ "ok": result.rows_affected > 0 }))))
}

#[delete(desc("Delete an admin model row"))]
async fn delete_row(
    State(orm): State<Orm>,
    State(auth): State<AdminAuth>,
    Header(header): Header<AdminHeaders>,
    Param(path): Param<RowPath>,
    Query(query): Query<VersionQuery>,
) -> HttpResult<Json<AdminJson>> {
    authorize(auth, &header)?;
    let info = admin_model(auth, &path.model, "delete")?;
    let primary = primary_key(info)?;
    let id = parse_id(&path.id, primary)?;
    let pk = quote_column(orm, primary.column_name)?;
    let mut binds = vec![id];
    let mut where_clause = format!("{pk} = ?");
    if let Some(version_column) = info.version_field {
        let version = query
            .version
            .as_deref()
            .filter(|value| !value.is_empty())
            .ok_or_else(|| client_error(400, "version is required for a versioned delete"))?;
        let version = version
            .parse::<i64>()
            .map_err(|_| client_error(400, "version must be an integer"))?;
        binds.push(Value::I64(version));
        where_clause.push_str(&format!(
            " AND {} = {}",
            quote_column(orm, version_column)?,
            "?"
        ));
    }
    let sql = if let Some(deleted_at) = info.soft_delete_field {
        let deleted_field = info
            .fields
            .iter()
            .find(|field| field.column_name == deleted_at)
            .ok_or_else(|| client_error(400, "invalid soft-delete metadata"))?;
        let mut assignments = vec![format!(
            "{} = {}",
            quote_column(orm, deleted_at)?,
            orm.dialect().current_value(deleted_field.field_type)
        )];
        if let Some(version) = info.version_field {
            let version = quote_column(orm, version)?;
            assignments.push(format!("{version} = {version} + 1"));
        }
        where_clause.push_str(&format!(" AND {} IS NULL", quote_column(orm, deleted_at)?));
        format!(
            "UPDATE {} SET {} WHERE {where_clause}",
            quote_table(orm, info)?,
            assignments.join(", ")
        )
    } else {
        format!(
            "DELETE FROM {} WHERE {where_clause}",
            quote_table(orm, info)?
        )
    };
    let result = orm
        .execute(&Statement::from_sql(orm.dialect(), &sql, binds).map_err(admin_error)?)
        .await
        .map_err(admin_error)?;
    Ok(Json(AdminJson(json!({ "ok": result.rows_affected > 0 }))))
}

fn admin_model<'a>(auth: &'a AdminAuth, name: &str, action: &str) -> HttpResult<&'a AdminModel> {
    auth.models
        .iter()
        .find(|model| {
            let info = model.database;
            let metadata = model.admin;
            (info.table_name == name || info.rust_name.eq_ignore_ascii_case(name))
                && match action {
                    "list" => metadata.list,
                    "detail" => metadata.detail || metadata.update,
                    "create" => metadata.create,
                    "update" => metadata.update,
                    "delete" => metadata.delete,
                    _ => false,
                }
        })
        .ok_or_else(|| client_error(404, "admin model or action not found"))
}

fn primary_key(info: &AdminModel) -> HttpResult<&AdminField> {
    let column = info
        .primary_key
        .ok_or_else(|| client_error(400, "admin model requires a primary key"))?;
    info.fields
        .iter()
        .find(|field| field.column_name == column)
        .ok_or_else(|| client_error(400, "invalid primary key metadata"))
}

fn field_is_admin_visible(field: &AdminField) -> bool {
    !field.admin.hidden
        && (field.admin.list
            || field.admin.detail
            || field.admin.create
            || field.admin.update
            || field.admin.filter
            || field.admin.search)
}

fn field_json(
    field: &AdminField,
    language: &str,
    i18ns: &[(String, HashMap<String, String>)],
) -> JsonValue {
    json!({
        "name": field.column_name,
        "label": localized_label(field.admin.label, field.admin.label_lang, field.rust_name, language, i18ns),
        "type": format!("{:?}", field.field_type).to_lowercase(),
        "order": field.admin.order,
        "nullable": field.nullable,
        "list": field.admin.list,
        "detail": field.admin.detail,
        "create": field.admin.create
            && !field.admin.readonly
            && !field.auto_create
            && !field.auto_update,
        "update": field.admin.update
            && !field.admin.readonly
            && !field.auto_create
            && !field.auto_update,
        "readonly": field.admin.readonly || field.auto_create || field.auto_update,
    })
}

fn row_to_json(row: &OrmRow, fields: &[&AdminField]) -> afastorm::Result<JsonValue> {
    let mut object = Map::new();
    for field in fields {
        let value = match field.field_type {
            FieldType::Bool => optional_field_json::<bool>(row, field)?,
            FieldType::I16 => optional_field_json::<i16>(row, field)?,
            FieldType::I32 => optional_field_json::<i32>(row, field)?,
            FieldType::I64 => optional_field_json::<i64>(row, field)?,
            FieldType::F32 => optional_field_json::<f32>(row, field)?,
            FieldType::F64 => optional_field_json::<f64>(row, field)?,
            FieldType::String => optional_field_json::<String>(row, field)?,
            FieldType::Date | FieldType::Time | FieldType::Timestamp | FieldType::TimestampTz => {
                optional_field_json::<String>(row, field)?
            }
            FieldType::Bytes => <Option<Vec<u8>> as OrmDecode>::decode(row, field.column_name)
                .map(|value| {
                    value.map_or(JsonValue::Null, |bytes| {
                        JsonValue::Array(bytes.into_iter().map(JsonValue::from).collect())
                    })
                })?,
            FieldType::Json => optional_field_json::<JsonValue>(row, field)?,
            field_type => {
                return Err(OrmError::UnsupportedFieldType(match field_type {
                    FieldType::Uuid => "UUID admin display",
                    FieldType::Numeric => "numeric admin display",
                    FieldType::BigNumeric => "big numeric admin display",
                    FieldType::IpAddr => "IP address admin display",
                    FieldType::IpNetwork => "IP network admin display",
                    FieldType::IpNet => "IP net admin display",
                    FieldType::MacAddress => "MAC address admin display",
                    FieldType::BitVec => "bit vector admin display",
                    FieldType::PostgresArray => "PostgreSQL array admin display",
                    FieldType::PostgresRange => "PostgreSQL range admin display",
                    _ => unreachable!(),
                }));
            }
        };
        object.insert(field.column_name.to_owned(), value);
    }
    Ok(JsonValue::Object(object))
}

fn optional_field_json<T>(row: &OrmRow, field: &FieldInfo) -> afastorm::Result<JsonValue>
where
    T: OrmDecode + Into<JsonValue>,
{
    Ok(<Option<T> as OrmDecode>::decode(row, field.column_name)?
        .map(Into::into)
        .unwrap_or(JsonValue::Null))
}

fn row_id(row: &OrmRow, field: &FieldInfo) -> HttpResult<String> {
    let id = match field.field_type {
        FieldType::Bool => decoded_id::<bool>(row, field)?,
        FieldType::I16 => decoded_id::<i16>(row, field)?,
        FieldType::I32 => decoded_id::<i32>(row, field)?,
        FieldType::I64 => decoded_id::<i64>(row, field)?,
        FieldType::F32 => decoded_id::<f32>(row, field)?,
        FieldType::F64 => decoded_id::<f64>(row, field)?,
        FieldType::String => decoded_id::<String>(row, field)?,
        _ => return Err(client_error(400, "unsupported primary key type")),
    };
    Ok(id)
}

fn decoded_id<T>(row: &OrmRow, field: &FieldInfo) -> HttpResult<String>
where
    T: OrmDecode + std::fmt::Display,
{
    <T as OrmDecode>::decode(row, field.column_name)
        .map(|id| id.to_string())
        .map_err(admin_error)
}

fn row_select(
    orm: &Orm,
    info: &AdminModel,
    fields: &[&AdminField],
    id: &str,
    include_soft_delete_filter: bool,
) -> HttpResult<Statement> {
    let primary = primary_key(info)?;
    let mut selected = fields
        .iter()
        .map(|field| select_column(orm, field))
        .collect::<afast::Result<Vec<_>>>()?;
    if !fields
        .iter()
        .any(|field| field.column_name == primary.column_name)
    {
        selected.push(quote_column(orm, primary.column_name)?);
    }
    if let Some(version) = info.version_field
        && !fields.iter().any(|field| field.column_name == version)
    {
        selected.push(quote_column(orm, version)?);
    }
    let mut sql = format!(
        "SELECT {} FROM {} WHERE {} = ?",
        selected.join(", "),
        quote_table(orm, info)?,
        quote_column(orm, primary.column_name)?
    );
    let mut binds = vec![parse_id(id, primary)?];
    if include_soft_delete_filter && let Some(deleted_at) = info.soft_delete_field {
        sql.push_str(&format!(" AND {} IS NULL", quote_column(orm, deleted_at)?));
    }
    Statement::from_sql(orm.dialect(), &sql, std::mem::take(&mut binds)).map_err(admin_error)
}

fn quote_table(orm: &Orm, info: &ModelInfo) -> HttpResult<String> {
    orm.dialect()
        .quote_identifier(info.table_name)
        .map_err(admin_error)
}

fn quote_column(orm: &Orm, name: &str) -> HttpResult<String> {
    orm.dialect().quote_identifier(name).map_err(admin_error)
}

fn select_column(orm: &Orm, field: &FieldInfo) -> HttpResult<String> {
    let column = quote_column(orm, field.column_name)?;
    if orm.dialect().name() == "postgres" {
        let cast = match field.field_type {
            FieldType::Json
            | FieldType::Date
            | FieldType::Time
            | FieldType::Timestamp
            | FieldType::TimestampTz => Some("TEXT"),
            _ => None,
        };
        if let Some(cast) = cast {
            return Ok(format!("{column}::{cast} AS {column}"));
        }
    }
    Ok(column)
}

fn field_value_placeholder(orm: &Orm, field: &FieldInfo) -> String {
    if orm.dialect().name() != "postgres" {
        return "?".to_owned();
    }
    match field.field_type {
        FieldType::Date => "CAST(? AS DATE)".to_owned(),
        FieldType::Time => "CAST(? AS TIME)".to_owned(),
        FieldType::Timestamp => "CAST(? AS TIMESTAMP)".to_owned(),
        FieldType::TimestampTz => "CAST(? AS TIMESTAMPTZ)".to_owned(),
        _ => "?".to_owned(),
    }
}

fn json_to_value(value: &JsonValue, field: &FieldInfo) -> HttpResult<Value> {
    if value.is_null() && field.nullable {
        return Ok(Value::Null(field.field_type));
    }
    let value = match field.field_type {
        FieldType::Bool => value.as_bool().map(Value::Bool),
        FieldType::I16 => value
            .as_i64()
            .and_then(|value| i16::try_from(value).ok())
            .map(Value::I16),
        FieldType::I32 => value
            .as_i64()
            .and_then(|value| i32::try_from(value).ok())
            .map(Value::I32),
        FieldType::I64 => value.as_i64().map(Value::I64),
        FieldType::F32 => value
            .as_f64()
            .map(|value| value as f32)
            .filter(|value| value.is_finite())
            .map(Value::F32),
        FieldType::F64 => value.as_f64().map(Value::F64),
        FieldType::String => value.as_str().map(|value| Value::String(value.to_owned())),
        FieldType::Date | FieldType::Time | FieldType::Timestamp | FieldType::TimestampTz => {
            value.as_str().map(|value| Value::String(value.to_owned()))
        }
        FieldType::Bytes => value
            .as_str()
            .map(|value| Value::Bytes(value.as_bytes().to_vec())),
        FieldType::Json => match value.as_str() {
            Some(raw) => Some(
                serde_json::from_str(raw)
                    .map(Value::Json)
                    .map_err(|_| client_error(400, "field value must contain valid JSON"))?,
            ),
            None => Some(Value::Json(value.clone())),
        },
        _ => {
            return Err(client_error(
                400,
                "admin editing is not supported for this field type",
            ));
        }
    };
    value.ok_or_else(|| client_error(400, "field value has the wrong type or is null"))
}

fn parse_id(id: &str, field: &FieldInfo) -> HttpResult<Value> {
    match field.field_type {
        FieldType::Bool => id
            .parse()
            .map(Value::Bool)
            .map_err(|_| client_error(400, "invalid id")),
        FieldType::I16 => id
            .parse()
            .map(Value::I16)
            .map_err(|_| client_error(400, "invalid id")),
        FieldType::I32 => id
            .parse()
            .map(Value::I32)
            .map_err(|_| client_error(400, "invalid id")),
        FieldType::I64 => id
            .parse()
            .map(Value::I64)
            .map_err(|_| client_error(400, "invalid id")),
        FieldType::F32 => id
            .parse()
            .map(Value::F32)
            .map_err(|_| client_error(400, "invalid id")),
        FieldType::F64 => id
            .parse()
            .map(Value::F64)
            .map_err(|_| client_error(400, "invalid id")),
        FieldType::String => Ok(Value::String(id.to_owned())),
        _ => Err(client_error(400, "unsupported primary key type")),
    }
}

fn admin_error(error: impl std::fmt::Display) -> afast::Error {
    afast::Error::custom(500, format!("afastorm admin request failed: {error}"))
}

fn client_error(code: i64, message: &'static str) -> afast::Error {
    afast::Error::custom(code, message)
}

#[cfg(test)]
mod tests {
    use super::{AdminAuth, AdminHeaders, AdminModelRegistry, authorize, localized_label, service};
    use afastorm::ModelRegistry;
    use std::collections::HashMap;

    #[test]
    fn admin_service_routes_use_the_configured_prefix() {
        let routes = service("/backoffice/").expect("valid admin URL");
        let admin_group = routes.handlers.first().expect("admin route group");
        assert_eq!(admin_group.name, "backoffice");
        assert!(admin_group.children.iter().any(|route| route.path == "/"));
        assert!(
            admin_group
                .children
                .iter()
                .any(|route| route.path == "/api/models")
        );
        assert!(
            admin_group
                .children
                .iter()
                .any(|route| route.path == "/api/token")
        );
    }

    #[cfg(test)]
    mod admin_auth_configuration_tests {
        use super::{AdminAuth, AdminHeaders, AdminModelRegistry, ModelRegistry, authorize};
        use std::sync::Arc;

        #[test]
        fn allows_requests_without_a_configured_token() {
            let auth = AdminAuth::new(
                None,
                None,
                None,
                "A".to_owned(),
                "Afast ORM".to_owned(),
                Vec::new(),
                &AdminModelRegistry::new(),
                &ModelRegistry::new(),
            )
            .expect("no-token admin configuration");
            assert!(
                authorize(
                    &auth,
                    &AdminHeaders {
                        authorization: None
                    }
                )
                .is_ok()
            );
        }

        #[test]
        fn empty_token_and_empty_locale_list_use_safe_defaults() {
            let auth = AdminAuth::new(
                Some("   ".to_owned()),
                None,
                None,
                String::new(),
                String::new(),
                Vec::new(),
                &AdminModelRegistry::new(),
                &ModelRegistry::new(),
            )
            .expect("empty values use defaults");
            assert_eq!(auth.auth_mode(), "none");
            assert_eq!(&*auth.logo_letter, "A");
            assert_eq!(&*auth.brand_text, "Afast ORM");
            assert_eq!(auth.i18ns[0].0, "zh-CN");
        }

        #[test]
        fn rejects_invalid_locale_configuration() {
            use std::collections::HashMap;

            let duplicate_locales = vec![
                ("en".to_owned(), HashMap::new()),
                ("en".to_owned(), HashMap::new()),
            ];
            assert!(
                AdminAuth::new(
                    None,
                    None,
                    None,
                    "A".to_owned(),
                    "Afast ORM".to_owned(),
                    duplicate_locales,
                    &AdminModelRegistry::new(),
                    &ModelRegistry::new(),
                )
                .is_err()
            );
        }

        #[test]
        fn custom_token_functions_are_paired_and_enforced() {
            let auth = AdminAuth::new(
                None,
                Some(Arc::new(|credential| {
                    (credential == "login-secret").then(|| "issued-token".to_owned())
                })),
                Some(Arc::new(|token| token == "issued-token")),
                "A".to_owned(),
                "Afast ORM".to_owned(),
                Vec::new(),
                &AdminModelRegistry::new(),
                &ModelRegistry::new(),
            )
            .expect("paired token functions");
            assert_eq!(
                auth.issue_token("login-secret").as_deref(),
                Some("issued-token")
            );
            assert!(
                authorize(
                    &auth,
                    &AdminHeaders {
                        authorization: Some("Bearer issued-token".to_owned())
                    }
                )
                .is_ok()
            );
            assert!(
                authorize(
                    &auth,
                    &AdminHeaders {
                        authorization: Some("Bearer invalid".to_owned())
                    }
                )
                .is_err()
            );
            assert!(
                AdminAuth::new(
                    None,
                    Some(Arc::new(|_| Some("token".to_owned()))),
                    None,
                    "A".to_owned(),
                    "Afast ORM".to_owned(),
                    Vec::new(),
                    &AdminModelRegistry::new(),
                    &ModelRegistry::new(),
                )
                .is_err()
            );
        }
    }

    #[test]
    fn rejects_invalid_admin_urls() {
        assert!(service("/").is_err());
        assert!(service("/admin?debug=1").is_err());
    }

    #[test]
    fn resolves_admin_labels_from_the_selected_and_default_languages() {
        let i18ns = vec![
            (
                "zh-CN".to_owned(),
                HashMap::from([("post.title".to_owned(), "标题".to_owned())]),
            ),
            (
                "en".to_owned(),
                HashMap::from([("post.title".to_owned(), "Title".to_owned())]),
            ),
        ];
        assert_eq!(
            localized_label(None, Some("post.title"), "title", "en", &i18ns),
            "Title"
        );
        assert_eq!(
            localized_label(None, Some("post.title"), "title", "fr", &i18ns),
            "标题"
        );
        assert_eq!(
            localized_label(None, Some("missing"), "title", "en", &i18ns),
            "title"
        );
        assert_eq!(
            localized_label(Some("Explicit"), Some("post.title"), "title", "en", &i18ns),
            "Explicit"
        );
    }

    #[test]
    fn requires_the_exact_bearer_token() {
        let auth = AdminAuth::new(
            Some("secret".to_owned()),
            None,
            None,
            "A".to_owned(),
            "Afast ORM".to_owned(),
            Vec::new(),
            &AdminModelRegistry::new(),
            &ModelRegistry::new(),
        )
        .expect("valid token");
        assert!(
            authorize(
                &auth,
                &AdminHeaders {
                    authorization: Some("Bearer secret".to_owned())
                }
            )
            .is_ok()
        );
        assert_eq!(
            authorize(
                &auth,
                &AdminHeaders {
                    authorization: Some("Bearer wrong".to_owned())
                }
            )
            .expect_err("wrong token must be rejected")
            .code(),
            401
        );
        assert_eq!(
            authorize(
                &auth,
                &AdminHeaders {
                    authorization: None
                }
            )
            .expect_err("missing token must be rejected")
            .code(),
            401
        );
    }
}
