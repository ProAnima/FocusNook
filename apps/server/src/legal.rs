pub const PRIVACY_POLICY_VERSION: &str = "2026-07-16";

#[derive(Clone)]
pub struct LegalIdentity {
    pub address: Option<String>,
    pub name: String,
    pub support_email: String,
    pub tax_id: Option<String>,
}

impl LegalIdentity {
    fn operator_description(&self) -> String {
        let mut description = escape(&self.name);
        if let Some(tax_id) = &self.tax_id {
            description.push_str(&format!(", ИНН {}", escape(tax_id)));
        }
        if let Some(address) = &self.address {
            description.push_str(&format!(", адрес: {}", escape(address)));
        }
        description
    }

    pub fn privacy_html(&self) -> String {
        page(
            "Политика конфиденциальности FocusNook",
            &format!(
                r#"
<h1>Политика конфиденциальности FocusNook</h1>
<p>Версия от 16 июля 2026 года.</p>
<h2>1. Оператор</h2>
<p>{operator}. По вопросам о персональных данных: <a href="mailto:{email}">{email}</a>.</p>
<h2>2. Какие данные обрабатываются</h2>
<ul>
  <li>имя и адрес электронной почты при создании аккаунта;</li>
  <li>стойкий хеш пароля — сам пароль на сервере не хранится;</li>
  <li>идентификаторы устройств, токены доступа и технические журналы синхронизации;</li>
  <li>зашифрованные планы, заметки, напоминания и вложения при включённой синхронизации.</li>
</ul>
<p>Микрофон используется только после действия пользователя для записи голосовой заметки или напоминания. Приложение не ведёт фоновую запись и не использует данные для рекламы.</p>
<h2>3. Цели и основания</h2>
<p>Данные нужны для регистрации и защиты аккаунта, синхронизации между устройствами, доставки напоминаний, диагностики ошибок и обеспечения безопасности. Основания обработки — согласие пользователя и исполнение пользовательского соглашения.</p>
<h2>4. Хранение и защита</h2>
<p>Обмен с сервером выполняется по HTTPS. Пароли хранятся как Argon2id-хеши, токены — только как криптографические хеши. Синхронизируемое содержимое шифруется клиентом, а сервер дополнительно шифрует его при хранении.</p>
<p>Данные хранятся, пока существует аккаунт, либо дольше только когда этого требует закон. Доступ инфраструктурных поставщиков ограничен объёмом, необходимым для работы и резервного копирования сервиса.</p>
<h2>5. Удаление и права пользователя</h2>
<p>В настройках FocusNook можно безвозвратно удалить аккаунт, серверные токены, операции синхронизации и вложения. Локальные данные на других устройствах удаляются вместе с приложением отдельно.</p>
<p>Для запроса доступа, исправления, удаления, ограничения обработки или отзыва согласия напишите на <a href="mailto:{email}">{email}</a>. Отзыв согласия не влияет на законность обработки до отзыва.</p>
<h2>6. Изменения</h2>
<p>При существенном изменении политики приложение запросит согласие с новой версией до операции, для которой оно необходимо.</p>
<p><a href="/terms">Пользовательское соглашение</a></p>
"#,
                operator = self.operator_description(),
                email = escape(&self.support_email),
            ),
        )
    }

    pub fn terms_html(&self) -> String {
        page(
            "Пользовательское соглашение FocusNook",
            &format!(
                r#"
<h1>Пользовательское соглашение FocusNook</h1>
<p>Редакция от 16 июля 2026 года.</p>
<h2>1. Стороны и принятие условий</h2>
<p>Правообладатель и оператор сервиса — {operator}. Устанавливая приложение, создавая аккаунт или используя синхронизацию, пользователь принимает это соглашение.</p>
<h2>2. Сервис</h2>
<p>FocusNook предоставляет локальные планы, заметки, голосовые записи и напоминания, а также необязательную синхронизацию между устройствами. Базовая версия распространяется бесплатно, если в карточке магазина прямо не указано иное.</p>
<h2>3. Аккаунт и безопасность</h2>
<p>Пользователь отвечает за точность адреса электронной почты, сохранность пароля и действия на своих устройствах. Запрещено пытаться получить доступ к чужим данным, нарушать работу сервиса или использовать его с нарушением закона и прав третьих лиц.</p>
<h2>4. Данные и удаление</h2>
<p>Обработка данных описана в <a href="/privacy">политике конфиденциальности</a>. Пользователь может удалить аккаунт в настройках приложения. Удаление серверной копии не удаляет локальные данные с других устройств автоматически.</p>
<h2>5. Интеллектуальные права</h2>
<p>Исключительные права на FocusNook, название, дизайн и собственные материалы принадлежат правообладателю. Пользователю предоставляется личное неисключительное право использовать установленную копию приложения по назначению.</p>
<h2>6. Доступность и ответственность</h2>
<p>Сервис развивается и может временно быть недоступен из-за обслуживания или обстоятельств вне разумного контроля правообладателя. В пределах, разрешённых применимым законом, правообладатель не отвечает за косвенные убытки и потерю данных; пользователь сохраняет все обязательные права потребителя.</p>
<h2>7. Изменения и контакты</h2>
<p>Новая редакция публикуется на этой странице. Существенные изменения применяются после уведомления или повторного согласия, когда оно требуется. Вопросы и претензии: <a href="mailto:{email}">{email}</a>.</p>
"#,
                operator = self.operator_description(),
                email = escape(&self.support_email),
            ),
        )
    }
}

fn page(title: &str, content: &str) -> String {
    format!(
        r#"<!doctype html><html lang="ru"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>{title}</title><style>:root{{font-family:system-ui,sans-serif;color:#20242b;background:#f6f4ef}}body{{margin:0}}main{{max-width:760px;margin:auto;padding:40px 20px 72px}}h1{{font-size:2rem}}h2{{margin-top:2rem}}p,li{{line-height:1.6}}a{{color:#405dcc}}.card{{background:#fff;border:1px solid #dedbd3;border-radius:16px;padding:24px}}</style></head><body><main><div class="card">{content}</div></main></body></html>"#,
        title = escape(title),
    )
}

fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legal_identity_is_html_escaped() {
        let identity = LegalIdentity {
            address: Some("<script>alert(1)</script>".to_string()),
            name: "A & B".to_string(),
            support_email: "support@example.com".to_string(),
            tax_id: Some("123".to_string()),
        };
        let html = identity.privacy_html();
        assert!(html.contains("A &amp; B"));
        assert!(!html.contains("<script>"));
    }

    #[test]
    fn legal_pages_render_without_tax_id_or_address() {
        let identity = LegalIdentity {
            address: None,
            name: "ProAnimaStudio".to_string(),
            support_email: "info@proanima.net".to_string(),
            tax_id: None,
        };
        let html = identity.privacy_html();
        assert!(html.contains("ProAnimaStudio."));
        assert!(html.contains("info@proanima.net"));
        assert!(!html.contains("ИНН"));
        assert!(!html.contains("адрес:"));
    }
}
