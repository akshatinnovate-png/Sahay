use serde_json::{json, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Calm,
    Warn,
    Stop,
}

#[derive(Debug, Clone)]
pub struct Verdict {
    pub level: Level,
    pub kind: &'static str,
    pub title: String,
    pub why: Vec<String>,
    pub kill: Vec<String>,
    pub close_page: bool,
    pub money_at_risk: bool,
}

pub const REMOTE_TOOLS: &[(&str, &str)] = &[
    ("anydesk", "AnyDesk"),
    ("teamviewer", "TeamViewer"),
    ("ultraviewer", "UltraViewer"),
    ("rustdesk", "RustDesk"),
    ("aeroadmin", "AeroAdmin"),
    ("supremo", "Supremo"),
    ("ammyy", "Ammyy Admin"),
    ("screenconnect", "ScreenConnect"),
    ("connectwisecontrol", "ScreenConnect"),
    ("getscreen", "Getscreen"),
    ("quickassist", "Quick Assist"),
    ("airdroid", "AirDroid"),
    ("iperius", "Iperius Remote"),
    ("zohoassist", "Zoho Assist"),
];
const CALL_APPS: &[(&str, &str)] = &[("whatsapp", "WhatsApp"), ("skype", "Skype"), ("zoom", "Zoom"), ("ms-teams", "Teams"), ("teams", "Teams"), ("telegram", "Telegram")];
const PAYMENT: &[&str] = &["otp", "one time password", "one-time password", "upi pin", "enter pin", "mpin", "cvv", "card number", "net banking", "netbanking", "transfer", "beneficiary", "debit card", "credit card", "ifsc", "account number"];
const PRESSURE: &[&str] = &["kyc", "update your", "blocked", "suspended", "expire", "refund", "cashback", "lottery", "prize", "verify your account", "electricity", "bill pending", "disconnected", "penalty"];
const TECH_SUPPORT: &[&str] = &["your computer is infected", "your pc is infected", "virus detected", "call microsoft", "microsoft support", "windows defender alert", "do not restart your computer", "your computer has been blocked", "call support immediately", "toll free", "security alert"];
const ARREST: &[&str] = &["digital arrest", "cbi", "narcotics", "money laundering", "arrest warrant", "customs", "enforcement directorate", "cyber crime department", "police station", "court order", "aadhaar misuse", "parcel"];
const GIFT: &[&str] = &["gift card", "google play card", "amazon pay card", "itunes card", "steam card", "bitcoin", "usdt", "crypto wallet"];

fn has(text: &str, words: &[&str]) -> Vec<String> {
    words.iter().filter(|w| text.contains(*w)).map(|w| (*w).to_string()).collect()
}

fn phone_number(text: &str) -> bool {
    let mut run = 0;
    for c in text.chars() {
        if c.is_ascii_digit() {
            run += 1;
            if run >= 10 {
                return true;
            }
        } else if c != ' ' && c != '-' {
            run = 0;
        }
    }
    false
}

pub fn assess(processes: &[String], titles: &[String], screen: &str) -> Verdict {
    let procs: Vec<String> = processes.iter().map(|p| p.to_lowercase().replace(".exe", "")).collect();
    let titles_l: Vec<String> = titles.iter().map(|t| t.to_lowercase()).collect();
    let text = format!("{} {}", titles_l.join(" "), screen.to_lowercase());
    let remote: Vec<(&str, &str)> = REMOTE_TOOLS.iter().copied().filter(|(exe, _)| procs.iter().any(|p| p.contains(exe))).collect();
    let calls: Vec<(&str, &str)> = CALL_APPS.iter().copied().filter(|(exe, name)| procs.iter().any(|p| p.starts_with(exe)) && titles_l.iter().any(|t| t.contains(&name.to_lowercase()) && (t.contains("call") || t.contains("video") || t.contains("meeting")))).collect();
    let payment = has(&text, PAYMENT);
    let pressure = has(&text, PRESSURE);
    let calm = Verdict { level: Level::Calm, kind: "none", title: String::new(), why: Vec::new(), kill: Vec::new(), close_page: false, money_at_risk: false };
    let kill_remote = || remote.iter().map(|(exe, _)| (*exe).to_string()).collect::<Vec<_>>();
    let names = |list: &[(&str, &str)]| list.iter().map(|(_, n)| *n).collect::<Vec<_>>().join(", ");

    if titles_l.iter().any(|t| (t.contains("uninstall") || t.contains("apps & features") || t.contains("installed apps")) && (t.contains("sahay") || t.contains("saathi"))) || (text.contains("uninstall") && (text.contains("sahay") || text.contains("saathi")) && (!remote.is_empty() || !calls.is_empty())) {
        return Verdict { level: Level::Warn, kind: "tamper", title: "Someone is trying to remove Sahay or Saathi".into(), why: vec!["The uninstall screen for Sahay or Saathi was opened".into()], ..calm };
    }
    if !remote.is_empty() && !payment.is_empty() {
        let mut why = vec![format!("{} lets someone control this computer from far away", names(&remote))];
        why.push(format!("At the same time the screen shows: {}", payment.join(", ")));
        return Verdict { level: Level::Stop, kind: "remote-payment", title: format!("{} was open while a bank or payment page was on screen", names(&remote)), why, kill: kill_remote(), close_page: true, money_at_risk: true };
    }
    let arrest = has(&text, ARREST);
    if !calls.is_empty() && !arrest.is_empty() && (arrest.len() >= 2 || text.contains("arrest")) {
        return Verdict {
            level: Level::Stop,
            kind: "digital-arrest",
            title: "A video call from someone pretending to be police or an officer".into(),
            why: vec![format!("A {} call is open", names(&calls)), format!("The screen mentions: {}", arrest.join(", ")), "Real police never arrest anyone on a video call or ask for money".into()],
            kill: calls.iter().map(|(exe, _)| (*exe).to_string()).collect(),
            close_page: false,
            money_at_risk: !payment.is_empty(),
        };
    }
    let tech = has(&text, TECH_SUPPORT);
    if tech.len() >= 2 || (!tech.is_empty() && phone_number(&text)) {
        return Verdict { level: Level::Stop, kind: "tech-support", title: "A fake \"your computer is infected\" warning".into(), why: vec![format!("The screen says: {}", tech.join(", ")), "Real companies never show a phone number to call in a pop-up".into()], kill: kill_remote(), close_page: true, money_at_risk: false };
    }
    if !remote.is_empty() && (!calls.is_empty() || !pressure.is_empty()) {
        let mut why = vec![format!("{} lets someone control this computer from far away", names(&remote))];
        if !calls.is_empty() {
            why.push(format!("A {} call is open at the same time", names(&calls)));
        }
        if !pressure.is_empty() {
            why.push(format!("The screen mentions: {}", pressure.join(", ")));
        }
        return Verdict { level: Level::Stop, kind: "remote-access", title: format!("Someone opened {} to control this computer", names(&remote)), why, kill: kill_remote(), close_page: false, money_at_risk: false };
    }
    let otp = payment.iter().any(|p| ["otp", "one time password", "one-time password", "upi pin", "mpin", "cvv", "enter pin"].contains(&p.as_str()));
    if otp && !pressure.is_empty() {
        return Verdict { level: Level::Stop, kind: "otp-page", title: "A page asking for an OTP or PIN with a scary message".into(), why: vec![format!("It asks for: {}", payment.join(", ")), format!("It says: {}", pressure.join(", ")), "Banks never ask for an OTP or PIN to update KYC, refund or unblock".into()], kill: Vec::new(), close_page: true, money_at_risk: true };
    }
    let gift = has(&text, GIFT);
    if !gift.is_empty() && (text.contains("pay") || text.contains("send") || text.contains("buy")) {
        return Verdict { level: Level::Warn, kind: "gift-card", title: "Someone wants payment in gift cards or crypto".into(), why: vec![format!("The screen mentions: {}", gift.join(", ")), "Government offices and companies never ask for gift cards or crypto".into()], ..calm };
    }
    calm
}

pub fn explain(kind: &str) -> (&'static str, [&'static str; 3]) {
    match kind {
        "remote-payment" | "remote-access" => ("I stopped this. Someone was trying to control this computer from far away. A real bank or company never asks you to install an app or share your screen. You are safe.", ["They call pretending to be from a bank, a company or the police.", "They ask you to install an app like AnyDesk so they can \"help\".", "Then they open your bank and move your money while you watch."]),
        "digital-arrest" => ("I ended this call. Real police never arrest anyone on a video call, and never ask for money to close a case. This is a scam. You are safe.", ["They video-call wearing a uniform and say your name is in a crime.", "They scare you and say you must stay on the call and tell no one.", "They ask you to send money to \"clear your name\"."]),
        "tech-support" => ("I closed this page. It is a fake warning: your computer is fine. Never call a number shown in a pop-up. You are safe.", ["A page suddenly says your computer has a virus.", "It shows a phone number to call for help.", "The \"helper\" asks for money or control of your computer."]),
        "otp-page" => ("I closed this page. A bank never asks for an OTP or PIN to update KYC, give a refund or unblock an account. You are safe.", ["A message says your account will be blocked or you have a refund.", "A link opens a page that looks like your bank.", "The OTP or PIN you type lets them take your money."]),
        "gift-card" => ("Wait. Nobody genuine asks to be paid in gift cards or crypto. Please talk to your family first.", ["Someone says you owe money or won a prize.", "They ask for gift cards or crypto because it cannot be traced.", "Once you share the card code, the money is gone."]),
        "tamper" => ("Someone is trying to remove Sahay or Saathi. Scammers ask this so nobody can stop them. I have told your family.", ["A caller asks you to close or uninstall a program.", "They say it is \"blocking\" their help.", "With it gone, nobody is watching while they take control."]),
        _ => ("This looks like a scam. Please talk to your family before doing anything.", ["Someone creates panic or excitement.", "They ask for a code, money or control of your computer.", "Then the money is gone."]),
    }
}

pub fn should_redact(line: &str) -> bool {
    let l = line.to_lowercase();
    let digits = l.chars().filter(char::is_ascii_digit).count();
    digits >= 4 || ["otp", "pin", "password", "cvv", "card", "account", "a/c", "ifsc", "upi", "@"].iter().any(|k| l.contains(k))
}

pub fn incident(v: &Verdict, actions: &[String], screenshot: Option<String>) -> Value {
    let (explain, how) = explain(v.kind);
    json!({
        "kind": v.kind,
        "title": v.title,
        "why": v.why,
        "explain": explain,
        "how": how,
        "actions": actions,
        "moneyAtRisk": v.money_at_risk,
        "screenshot": screenshot,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(list: &[&str]) -> Vec<String> {
        list.iter().map(|x| (*x).to_string()).collect()
    }

    #[test]
    fn remote_control_with_a_bank_page_is_stopped_and_money_is_at_risk() {
        let v = assess(&s(&["explorer.exe", "AnyDesk.exe", "chrome.exe"]), &s(&["SBI Online - Google Chrome"]), "Enter the OTP sent to your mobile");
        assert_eq!((v.level, v.kind, v.money_at_risk, v.close_page), (Level::Stop, "remote-payment", true, true));
        assert_eq!(v.kill, vec!["anydesk"]);
        assert!(v.title.contains("AnyDesk"));
    }

    #[test]
    fn remote_control_during_a_call_or_a_kyc_story_is_stopped() {
        let v = assess(&s(&["TeamViewer.exe", "WhatsApp.exe"]), &s(&["WhatsApp - Voice call"]), "");
        assert_eq!((v.level, v.kind), (Level::Stop, "remote-access"));
        assert_eq!(v.why.len(), 2);
        let k = assess(&s(&["rustdesk.exe"]), &s(&["Notepad"]), "Your KYC is pending, update your details");
        assert_eq!(k.kind, "remote-access");
        assert_eq!(assess(&s(&["anydesk.exe"]), &s(&["Notepad"]), "").level, Level::Calm);
    }

    #[test]
    fn digital_arrest_calls_are_ended() {
        let v = assess(&s(&["Skype.exe"]), &s(&["Skype - Video call"]), "This is CBI. A parcel in your name had narcotics. You are under digital arrest.");
        assert_eq!((v.level, v.kind), (Level::Stop, "digital-arrest"));
        assert_eq!(v.kill, vec!["skype"]);
        assert_eq!(assess(&s(&["Skype.exe"]), &s(&["Skype - Video call"]), "Happy birthday Aaji! The parcel arrived").level, Level::Calm);
    }

    #[test]
    fn fake_virus_warnings_otp_traps_gift_cards_and_tampering_are_caught() {
        assert_eq!(assess(&[], &s(&["Security Alert - Edge"]), "Your computer is infected! Call Microsoft support 1800 123 4567").kind, "tech-support");
        assert_eq!(assess(&[], &s(&["x"]), "Virus detected. Do not restart your computer").kind, "tech-support");
        assert_eq!(assess(&[], &s(&["Edge"]), "Virus detected").level, Level::Calm);
        let otp = assess(&[], &s(&["bank-kyc-update.xyz"]), "Your account will be blocked. Update your KYC. Enter OTP");
        assert_eq!((otp.kind, otp.money_at_risk), ("otp-page", true));
        assert_eq!(assess(&[], &s(&["HDFC NetBanking"]), "Enter OTP to log in").level, Level::Calm);
        let gift = assess(&[], &s(&["Mail"]), "Please buy Google Play card and send the code");
        assert_eq!((gift.level, gift.kind), (Level::Warn, "gift-card"));
        assert_eq!(assess(&[], &s(&["Uninstall Sahay"]), "").kind, "tamper");
        assert_eq!(assess(&s(&["anydesk.exe"]), &s(&["Control Panel"]), "Uninstall Saathi").kind, "tamper");
    }

    #[test]
    fn explanations_reports_and_redaction() {
        for kind in ["remote-payment", "remote-access", "digital-arrest", "tech-support", "otp-page", "gift-card", "tamper", "x"] {
            let (say, how) = explain(kind);
            assert!(!say.is_empty() && how.iter().all(|h| !h.is_empty()));
        }
        assert!(should_redact("Account 1234 5678"));
        assert!(should_redact("Your UPI PIN"));
        assert!(should_redact("riya@okaxis"));
        assert!(!should_redact("Welcome back"));
        let v = assess(&s(&["anydesk.exe"]), &s(&["Bank"]), "net banking transfer");
        let r = incident(&v, &["Closed AnyDesk".to_string()], None);
        assert_eq!(r["kind"], "remote-payment");
        assert_eq!(r["how"].as_array().unwrap().len(), 3);
        assert!(r["screenshot"].is_null());
        assert!(phone_number("call 1800-123-4567") && !phone_number("12 34"));
    }
}
