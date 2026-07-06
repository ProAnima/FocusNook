use crate::error::{AppError, AppResult};
use base64::Engine;
use rand::RngCore;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use subtle::ConstantTimeEq;

const SESSION_TTL: Duration = Duration::from_secs(60 * 60 * 8);
const LOGIN_WINDOW: Duration = Duration::from_secs(60 * 10);
const LOCKOUT: Duration = Duration::from_secs(60 * 15);
const MAX_ATTEMPTS: u32 = 5;

#[derive(Default)]
pub struct AdminWebState {
    attempts: Mutex<HashMap<String, LoginAttempt>>,
    sessions: Mutex<HashMap<String, Instant>>,
}

#[derive(Clone)]
struct LoginAttempt {
    count: u32,
    first_seen: Instant,
    locked_until: Option<Instant>,
}

impl Default for LoginAttempt {
    fn default() -> Self {
        Self {
            count: 0,
            first_seen: Instant::now(),
            locked_until: None,
        }
    }
}

impl AdminWebState {
    pub fn login(&self, ip: &str, password: &str, expected_password: &str) -> AppResult<String> {
        self.prune();
        if self.is_locked(ip)? {
            return Err(AppError::TooManyRequests(
                "too many login attempts, try later".to_string(),
            ));
        }

        if !constant_time_eq(password.trim(), expected_password) {
            self.record_failure(ip)?;
            return Err(AppError::Unauthorized);
        }

        self.clear_attempts(ip)?;
        let token = issue_session_token();
        let expires_at = Instant::now() + SESSION_TTL;
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| AppError::Internal("admin session lock failed".to_string()))?;
        sessions.insert(token.clone(), expires_at);
        Ok(token)
    }

    pub fn authorize(&self, token: &str) -> AppResult<()> {
        self.prune();
        let sessions = self
            .sessions
            .lock()
            .map_err(|_| AppError::Internal("admin session lock failed".to_string()))?;
        match sessions.get(token.trim()) {
            Some(expires_at) if *expires_at > Instant::now() => Ok(()),
            _ => Err(AppError::Unauthorized),
        }
    }

    fn is_locked(&self, ip: &str) -> AppResult<bool> {
        let attempts = self
            .attempts
            .lock()
            .map_err(|_| AppError::Internal("admin attempt lock failed".to_string()))?;
        Ok(attempts
            .get(ip)
            .and_then(|attempt| attempt.locked_until)
            .is_some_and(|until| until > Instant::now()))
    }

    fn record_failure(&self, ip: &str) -> AppResult<()> {
        let mut attempts = self
            .attempts
            .lock()
            .map_err(|_| AppError::Internal("admin attempt lock failed".to_string()))?;
        let attempt = attempts.entry(ip.to_string()).or_default();
        if Instant::now().duration_since(attempt.first_seen) > LOGIN_WINDOW {
            *attempt = LoginAttempt::default();
        }
        attempt.count += 1;
        if attempt.count >= MAX_ATTEMPTS {
            attempt.locked_until = Some(Instant::now() + LOCKOUT);
        }
        Ok(())
    }

    fn clear_attempts(&self, ip: &str) -> AppResult<()> {
        let mut attempts = self
            .attempts
            .lock()
            .map_err(|_| AppError::Internal("admin attempt lock failed".to_string()))?;
        attempts.remove(ip);
        Ok(())
    }

    fn prune(&self) {
        let now = Instant::now();
        if let Ok(mut sessions) = self.sessions.lock() {
            sessions.retain(|_, expires_at| *expires_at > now);
        }
        if let Ok(mut attempts) = self.attempts.lock() {
            attempts.retain(|_, attempt| {
                attempt.locked_until.is_some_and(|until| until > now)
                    || now.duration_since(attempt.first_seen) <= LOGIN_WINDOW
            });
        }
    }
}

fn issue_session_token() -> String {
    let mut bytes = [0_u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

fn constant_time_eq(left: &str, right: &str) -> bool {
    left.as_bytes().ct_eq(right.as_bytes()).into()
}

pub const ADMIN_HTML: &str = r#"<!doctype html>
<html lang="ru">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>FocusNook Sync Console</title>
  <link rel="icon" type="image/svg+xml" href="data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 64 64'%3E%3Crect width='64' height='64' rx='16' fill='%23141b32'/%3E%3Crect x='14' y='15' width='36' height='34' rx='8' fill='none' stroke='%2367f5df' stroke-width='4'/%3E%3Cpath d='M22 25h20M22 34h12' stroke='%238b7dff' stroke-width='4' stroke-linecap='round'/%3E%3Ccircle cx='44' cy='43' r='8' fill='%23ffbd63'/%3E%3C/svg%3E">
  <style>
    :root{color-scheme:dark;--bg:#0e1324;--panel:rgba(20,27,50,.84);--panel2:rgba(34,43,78,.68);--text:#eef3ff;--muted:#9eabc9;--line:rgba(255,255,255,.11);--accent:#67f5df;--warn:#ffc66d;--bad:#ff7b91;--good:#69e798;--shadow:0 24px 80px rgba(0,0,0,.34)}
    [data-theme=light]{color-scheme:light;--bg:#edf2f7;--panel:rgba(255,255,255,.88);--panel2:rgba(240,245,250,.86);--text:#162035;--muted:#61708c;--line:rgba(32,46,78,.13);--accent:#0f9d8f;--warn:#a86703;--bad:#c63251;--good:#158a4c;--shadow:0 24px 70px rgba(64,84,118,.17)}
    *{box-sizing:border-box}body{margin:0;min-height:100vh;background:radial-gradient(circle at 12% 12%,rgba(103,245,223,.20),transparent 30%),radial-gradient(circle at 82% 18%,rgba(147,122,255,.20),transparent 34%),linear-gradient(140deg,#111735,var(--bg));color:var(--text);font:14px/1.45 Inter,ui-sans-serif,system-ui,-apple-system,"Segoe UI",sans-serif;letter-spacing:0}
    [data-theme=light] body{background:radial-gradient(circle at 10% 10%,rgba(34,186,171,.18),transparent 34%),radial-gradient(circle at 80% 16%,rgba(91,112,230,.14),transparent 34%),linear-gradient(140deg,#f7f9fc,var(--bg))}
    .app{width:min(1180px,calc(100vw - 32px));margin:0 auto;padding:28px 0 40px}.top{display:flex;align-items:center;justify-content:space-between;gap:16px;margin-bottom:20px}.brand{display:flex;align-items:center;gap:12px}.logo{width:38px;height:38px;border-radius:12px;background:linear-gradient(135deg,#ffbd63,#6ff5df);display:grid;place-items:center;color:#162035;font-weight:900;box-shadow:0 12px 32px rgba(0,0,0,.24)}h1{font-size:22px;line-height:1.1;margin:0}p{margin:0}.muted{color:var(--muted)}.actions{display:flex;gap:8px;align-items:center;flex-wrap:wrap}button,select,input{font:inherit}button,.select{border:1px solid var(--line);background:var(--panel2);color:var(--text);border-radius:10px;padding:9px 12px;min-height:38px}button{cursor:pointer}button.primary{background:linear-gradient(135deg,var(--accent),#8b7dff);border-color:transparent;color:#07111f;font-weight:800}.grid{display:grid;grid-template-columns:repeat(4,minmax(0,1fr));gap:12px}.card{background:var(--panel);border:1px solid var(--line);border-radius:16px;box-shadow:var(--shadow);backdrop-filter:blur(18px) saturate(1.08)}.metric{padding:16px}.metric span{display:block;color:var(--muted);font-size:12px}.metric strong{display:block;font-size:26px;margin-top:8px}.panel{margin-top:12px;padding:16px}.table-wrap{overflow:auto;border:1px solid var(--line);border-radius:12px}table{width:100%;border-collapse:collapse;min-width:820px}th,td{text-align:left;padding:11px 12px;border-bottom:1px solid var(--line);white-space:nowrap}th{color:var(--muted);font-size:12px;font-weight:700;background:rgba(255,255,255,.035)}td strong{font-weight:800}.bar{height:7px;border-radius:999px;background:var(--panel2);overflow:hidden;min-width:110px}.bar i{display:block;height:100%;background:linear-gradient(90deg,var(--accent),#8b7dff);border-radius:inherit}.login{width:min(420px,calc(100vw - 32px));margin:12vh auto 0;padding:22px}.login h2{margin:0 0 8px;font-size:22px}.login form{display:grid;gap:10px;margin-top:18px}.login input{width:100%;border:1px solid var(--line);background:var(--panel2);color:var(--text);border-radius:10px;padding:12px}.error{color:var(--bad);min-height:20px}.pill{display:inline-flex;align-items:center;gap:6px;padding:4px 8px;border-radius:999px;background:var(--panel2);color:var(--muted);font-size:12px}.ok{color:var(--good)}.bad{color:var(--bad)}.hidden{display:none!important}@media(max-width:780px){.grid{grid-template-columns:repeat(2,minmax(0,1fr))}.top{align-items:flex-start;flex-direction:column}.app{width:min(100vw - 20px,1180px);padding-top:14px}.metric strong{font-size:22px}}
  </style>
</head>
<body>
  <section id="login" class="login card">
    <div class="brand"><div class="logo">F</div><div><h2 data-i18n="loginTitle">FocusNook Console</h2><p class="muted" data-i18n="loginHint">Enter the secondary password.</p></div></div>
    <form id="loginForm"><input id="password" type="password" autocomplete="current-password" data-i18n-placeholder="password" placeholder="Password"><button class="primary" data-i18n="signIn">Sign in</button><div id="loginError" class="error"></div></form>
  </section>
  <main id="app" class="app hidden">
    <header class="top"><div class="brand"><div class="logo">F</div><div><h1>FocusNook Sync</h1><p class="muted" data-i18n="subtitle">Private monitoring console</p></div></div><div class="actions"><span id="updated" class="pill"></span><select id="lang" class="select"></select><button id="themeBtn" data-i18n="theme">Theme</button><button id="refreshBtn" class="primary" data-i18n="refresh">Refresh</button></div></header>
    <section class="grid">
      <div class="card metric"><span data-i18n="users">Users</span><strong id="mUsers">0</strong></div>
      <div class="card metric"><span data-i18n="devices">Devices</span><strong id="mDevices">0</strong></div>
      <div class="card metric"><span data-i18n="storage">Storage</span><strong id="mStorage">0 B</strong></div>
      <div class="card metric"><span data-i18n="traffic">Traffic</span><strong id="mTraffic">0 B</strong></div>
    </section>
    <section class="card panel"><div class="table-wrap"><table><thead><tr><th data-i18n="user">User</th><th data-i18n="status">Status</th><th data-i18n="devices">Devices</th><th data-i18n="operations">Ops</th><th data-i18n="blobs">Blobs</th><th data-i18n="storage">Storage</th><th data-i18n="inbound">In</th><th data-i18n="outbound">Out</th><th data-i18n="lastSeen">Last seen</th></tr></thead><tbody id="usersBody"></tbody></table></div></section>
  </main>
  <script>
    const I={ru:{loginTitle:"FocusNook Console",loginHint:"Введите вторичный пароль.",password:"Пароль",signIn:"Войти",subtitle:"Приватный мониторинг синхронизации",theme:"Тема",refresh:"Обновить",users:"Пользователи",devices:"Устройства",storage:"Место",traffic:"Трафик",user:"Пользователь",status:"Статус",operations:"Операции",blobs:"Файлы",inbound:"Входящий",outbound:"Исходящий",lastSeen:"Активность",active:"Активен",disabled:"Отключен",updated:"Обновлено",error:"Ошибка входа"},en:{loginTitle:"FocusNook Console",loginHint:"Enter the secondary password.",password:"Password",signIn:"Sign in",subtitle:"Private sync monitoring",theme:"Theme",refresh:"Refresh",users:"Users",devices:"Devices",storage:"Storage",traffic:"Traffic",user:"User",status:"Status",operations:"Ops",blobs:"Blobs",inbound:"In",outbound:"Out",lastSeen:"Last seen",active:"Active",disabled:"Disabled",updated:"Updated",error:"Sign-in failed"},es:{loginTitle:"Consola FocusNook",loginHint:"Introduce la contraseña secundaria.",password:"Contraseña",signIn:"Entrar",subtitle:"Monitor privado de sincronización",theme:"Tema",refresh:"Actualizar",users:"Usuarios",devices:"Dispositivos",storage:"Espacio",traffic:"Tráfico",user:"Usuario",status:"Estado",operations:"Ops",blobs:"Archivos",inbound:"Entrada",outbound:"Salida",lastSeen:"Actividad",active:"Activo",disabled:"Desactivado",updated:"Actualizado",error:"Error de acceso"},de:{loginTitle:"FocusNook Konsole",loginHint:"Sekundäres Passwort eingeben.",password:"Passwort",signIn:"Anmelden",subtitle:"Privates Sync-Monitoring",theme:"Theme",refresh:"Aktualisieren",users:"Benutzer",devices:"Geräte",storage:"Speicher",traffic:"Traffic",user:"Benutzer",status:"Status",operations:"Ops",blobs:"Dateien",inbound:"Eingang",outbound:"Ausgang",lastSeen:"Aktivität",active:"Aktiv",disabled:"Deaktiviert",updated:"Aktualisiert",error:"Anmeldung fehlgeschlagen"},fr:{loginTitle:"Console FocusNook",loginHint:"Saisissez le mot de passe secondaire.",password:"Mot de passe",signIn:"Connexion",subtitle:"Supervision privée",theme:"Thème",refresh:"Actualiser",users:"Utilisateurs",devices:"Appareils",storage:"Stockage",traffic:"Trafic",user:"Utilisateur",status:"Statut",operations:"Ops",blobs:"Fichiers",inbound:"Entrant",outbound:"Sortant",lastSeen:"Activité",active:"Actif",disabled:"Désactivé",updated:"Mis à jour",error:"Connexion échouée"},pt:{loginTitle:"Console FocusNook",loginHint:"Digite a senha secundária.",password:"Senha",signIn:"Entrar",subtitle:"Monitoramento privado",theme:"Tema",refresh:"Atualizar",users:"Usuários",devices:"Dispositivos",storage:"Armazenamento",traffic:"Tráfego",user:"Usuário",status:"Status",operations:"Ops",blobs:"Arquivos",inbound:"Entrada",outbound:"Saída",lastSeen:"Atividade",active:"Ativo",disabled:"Desativado",updated:"Atualizado",error:"Falha no login"},zh:{loginTitle:"FocusNook 控制台",loginHint:"输入二级密码。",password:"密码",signIn:"登录",subtitle:"私有同步监控",theme:"主题",refresh:"刷新",users:"用户",devices:"设备",storage:"存储",traffic:"流量",user:"用户",status:"状态",operations:"操作",blobs:"文件",inbound:"入站",outbound:"出站",lastSeen:"最近活动",active:"启用",disabled:"停用",updated:"已更新",error:"登录失败"},ja:{loginTitle:"FocusNook コンソール",loginHint:"二次パスワードを入力してください。",password:"パスワード",signIn:"ログイン",subtitle:"プライベート同期監視",theme:"テーマ",refresh:"更新",users:"ユーザー",devices:"デバイス",storage:"容量",traffic:"通信量",user:"ユーザー",status:"状態",operations:"操作",blobs:"ファイル",inbound:"受信",outbound:"送信",lastSeen:"最終活動",active:"有効",disabled:"無効",updated:"更新済み",error:"ログイン失敗"},ko:{loginTitle:"FocusNook 콘솔",loginHint:"보조 비밀번호를 입력하세요.",password:"비밀번호",signIn:"로그인",subtitle:"비공개 동기화 모니터링",theme:"테마",refresh:"새로고침",users:"사용자",devices:"기기",storage:"저장공간",traffic:"트래픽",user:"사용자",status:"상태",operations:"작업",blobs:"파일",inbound:"수신",outbound:"송신",lastSeen:"최근 활동",active:"활성",disabled:"비활성",updated:"업데이트",error:"로그인 실패"},hi:{loginTitle:"FocusNook Console",loginHint:"दूसरा पासवर्ड दर्ज करें.",password:"पासवर्ड",signIn:"साइन इन",subtitle:"निजी सिंक मॉनिटरिंग",theme:"थीम",refresh:"रीफ्रेश",users:"यूज़र",devices:"डिवाइस",storage:"स्टोरेज",traffic:"ट्रैफिक",user:"यूज़र",status:"स्थिति",operations:"ऑप्स",blobs:"फाइलें",inbound:"इन",outbound:"आउट",lastSeen:"गतिविधि",active:"सक्रिय",disabled:"बंद",updated:"अपडेट",error:"लॉगिन विफल"}};
    const langs=Object.keys(I);let lang=localStorage.fnLang||navigator.language.slice(0,2);if(!I[lang])lang="ru";let token=localStorage.fnAdminToken||"";const $=id=>document.getElementById(id);function t(k){return I[lang][k]||I.en[k]||k}function applyLang(){document.querySelectorAll("[data-i18n]").forEach(e=>e.textContent=t(e.dataset.i18n));document.querySelectorAll("[data-i18n-placeholder]").forEach(e=>e.placeholder=t(e.dataset.i18nPlaceholder));$("lang").innerHTML=langs.map(l=>`<option value="${l}" ${l===lang?"selected":""}>${l.toUpperCase()}</option>`).join("")}function fmt(n){if(!n)return"0 B";let u=["B","KB","MB","GB","TB"],i=0;while(n>=1024&&i<u.length-1){n/=1024;i++}return `${n.toFixed(i?1:0)} ${u[i]}`}function showApp(){ $("login").classList.add("hidden");$("app").classList.remove("hidden")}async function login(p){let r=await fetch("/v1/admin/web/login",{method:"POST",headers:{"Content-Type":"application/json"},body:JSON.stringify({password:p})});if(!r.ok)throw 0;let j=await r.json();token=j.sessionToken;localStorage.fnAdminToken=token;showApp();await load()}async function load(){let r=await fetch("/v1/admin/monitor",{headers:{Authorization:`Bearer ${token}`}});if(r.status===401){localStorage.removeItem("fnAdminToken");$("app").classList.add("hidden");$("login").classList.remove("hidden");return}let d=await r.json();$("mUsers").textContent=d.summary.users;$("mDevices").textContent=d.summary.devices;$("mStorage").textContent=fmt(d.summary.storageBytes);$("mTraffic").textContent=fmt(d.summary.inboundBytes+d.summary.outboundBytes);$("updated").textContent=`${t("updated")}: ${new Date(d.generatedAt).toLocaleTimeString()}`;$("usersBody").innerHTML=d.users.map(u=>`<tr><td><strong>${u.displayName}</strong><br><span class="muted">${u.userId.slice(0,8)}</span></td><td><span class="pill ${u.disabled?"bad":"ok"}">${u.disabled?t("disabled"):t("active")}</span></td><td>${u.devices}</td><td>${u.operations}</td><td>${u.blobs}</td><td><div class="bar"><i style="width:${Math.min(100,u.storageBytes/(d.summary.storageBytes||1)*100)}%"></i></div>${fmt(u.storageBytes)}</td><td>${fmt(u.inboundBytes)}</td><td>${fmt(u.outboundBytes)}</td><td>${u.lastSeenAt?new Date(u.lastSeenAt).toLocaleString():"-"}</td></tr>`).join("")}
    $("loginForm").addEventListener("submit",e=>{e.preventDefault();$("loginError").textContent="";login($("password").value).catch(()=>$("loginError").textContent=t("error"))});$("refreshBtn").onclick=load;$("lang").onchange=e=>{lang=e.target.value;localStorage.fnLang=lang;applyLang();load()};$("themeBtn").onclick=()=>{let next=document.documentElement.dataset.theme==="light"?"dark":"light";document.documentElement.dataset.theme=next;localStorage.fnTheme=next};document.documentElement.dataset.theme=localStorage.fnTheme||"dark";applyLang();if(token){showApp();load()}
  </script>
</body>
</html>"#;
