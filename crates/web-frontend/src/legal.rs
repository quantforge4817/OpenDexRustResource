use leptos::prelude::*;

/// Legal page containing all regulatory compliance sections
#[allow(non_snake_case)]
pub fn LegalPage() -> impl IntoView {
    view! {
        <div class="container">
            <LegalDisclaimerSection />
            <RiskDisclosureSection />
            <JurisdictionNoticeSection />
            <PrivacyPolicySection />
            <TermsOfUseSection />
            <ContactSection />
            <FooterSection />
        </div>
    }
}

/// Legal Disclaimer section
#[allow(non_snake_case)]
fn LegalDisclaimerSection() -> impl IntoView {
    view! {
        <section id="legal-disclaimer" class="section legal-section">
            <h2>"Legal Disclaimer"</h2>
            <div class="legal-content">
                <div class="legal-warning-box">
                    <p class="warning-icon">"⚠️"</p>
                    <p class="warning-text">
                        "This website and the OpenDex software are provided for informational and educational purposes only. Nothing contained herein constitutes financial, legal, tax, or investment advice."
                    </p>
                </div>
                <p>
                    "The OpenDex project, its contributors, and affiliates make no representations or warranties regarding the legality of using OpenDex in any particular jurisdiction. Users are solely responsible for determining whether their use of OpenDex complies with applicable laws and regulations in their jurisdiction."
                </p>
                <p>
                    "OpenDex is experimental software provided 'as is' without warranty of any kind, express or implied. The project does not guarantee availability, reliability, suitability, or accuracy of any information, software, or related graphics."
                </p>
                <p>
                    "By using OpenDex, you acknowledge and agree that:"
                </p>
                <ul class="legal-list">
                    <li>"You are solely responsible for your own financial decisions and risk assessment."</li>
                    <li>"You have consulted with qualified legal and financial advisors in your jurisdiction."</li>
                    <li>"You understand the technical risks associated with cryptocurrency trading and atomic swaps."</li>
                    <li>"The OpenDex project and its contributors shall not be liable for any damages arising from your use of the software."</li>
                </ul>
            </div>
        </section>
    }
}

/// Risk Disclosure section
#[allow(non_snake_case)]
fn RiskDisclosureSection() -> impl IntoView {
    view! {
        <section id="risk-disclosure" class="section legal-section">
            <h2>"Risk Disclosure"</h2>
            <div class="legal-content">
                <h3>"Cryptocurrency Trading Risks"</h3>
                <p>
                    "Trading cryptocurrency involves substantial risk of loss and is not suitable for every investor. The value of cryptocurrencies can fluctuate widely, and you may lose some or all of your invested capital. You should not invest money that you cannot afford to lose."
                </p>
                <h3>"Monero (XMR) Regulatory Risks"</h3>
                <p>
                    "Monero (XMR) may be restricted or prohibited in certain jurisdictions. Under EU MiCA (Markets in Crypto-Assets Regulation) Article 76, crypto-assets with privacy features may be subject to additional restrictions or bans. Similar restrictions may apply in other jurisdictions including the United Kingdom, United States, and others."
                </p>
                <p>
                    "Users are responsible for understanding the regulatory status of Monero and other cryptocurrencies in their jurisdiction before engaging in any trading activity."
                </p>
                <h3>"Technical Risks"</h3>
                <ul class="legal-list">
                    <li>"Atomic swaps are experimental technology. Smart contract and cryptographic failures may result in permanent loss of funds."</li>
                    <li>"Counterparty risk: the other party in a swap may fail to complete the transaction."</li>
                    <li>"Network risk: P2P network failures or connectivity issues may prevent trade execution."</li>
                    <li>"Software risk: bugs or vulnerabilities in the OpenDex software may expose funds to risk."</li>
                    <li>"Key management risk: loss of private keys results in permanent loss of access to funds."</li>
                </ul>
                <h3>"Market Risks"</h3>
                <p>
                    "Cryptocurrency markets are highly volatile and illiquid compared to traditional financial markets. Prices can change rapidly and dramatically. Past performance is not indicative of future results."
                </p>
            </div>
        </section>
    }
}

/// Jurisdiction Notice section
#[allow(non_snake_case)]
fn JurisdictionNoticeSection() -> impl IntoView {
    view! {
        <section id="jurisdiction" class="section legal-section">
            <h2>"Jurisdiction Notice"</h2>
            <div class="legal-content">
                <p>
                    "OpenDex availability and legality may vary by jurisdiction. The project does not restrict access by geographic location, but users bear full responsibility for ensuring their use complies with local laws and regulations."
                </p>
                <h3>"European Union"</h3>
                <p>
                    "The EU Markets in Crypto-Assets Regulation (MiCA) establishes a comprehensive framework for crypto-asset services. OpenDex users in the EU should verify that their use of the platform complies with MiCA requirements, including but not limited to Articles 76 (privacy coins) and the Crypto-Asset Service Provider (CASP) regulations."
                </p>
                <h3>"United Kingdom"</h3>
                <p>
                    "The UK Financial Conduct Authority (FCA) applies substance-based, fact-specific perimeter analysis to crypto-asset activities. Users in the UK should consult the FCA's guidance on crypto-assets and ensure their activities comply with applicable regulations."
                </p>
                <h3>"Other Jurisdictions"</h3>
                <p>
                    "Users in other jurisdictions should consult qualified legal counsel to determine the regulatory status of cryptocurrency trading, atomic swaps, and privacy-focused cryptocurrencies in their location."
                </p>
                <div class="legal-warning-box">
                    <p class="warning-text">
                        "If you are uncertain about the legality of using OpenDex in your jurisdiction, do not use the software until you have obtained appropriate legal advice."
                    </p>
                </div>
            </div>
        </section>
    }
}

/// Privacy Policy section
#[allow(non_snake_case)]
fn PrivacyPolicySection() -> impl IntoView {
    view! {
        <section id="privacy" class="section legal-section">
            <h2>"Privacy Policy"</h2>
            <div class="legal-content">
                <p class="last-updated">"Last updated: October 2026"</p>
                <h3>"Website Data Collection"</h3>
                <p>
                    "The OpenDex website is a fully static site. It does not collect, store, or process any personal data. There are no tracking cookies, analytics scripts, or data collection mechanisms on this website."
                </p>
                <h3>"What We Do NOT Collect"</h3>
                <ul class="legal-list">
                    <li>"No personal identification information (name, email, address)"</li>
                    <li>"No browsing behavior or analytics data"</li>
                    <li>"No IP addresses or device fingerprints"</li>
                    <li>"No KYC or identity verification data"</li>
                    <li>"No transaction history or trading data"</li>
                    <li>"No cookies or tracking pixels"</li>
                </ul>
                <h3>"P2P Protocol Privacy"</h3>
                <p>
                    "The OpenDex P2P protocol is designed with privacy as a core principle:"
                </p>
                <ul class="legal-list">
                    <li>"All P2P communications are encrypted using the Noise protocol"</li>
                    <li>"No central server stores user data or transaction history"</li>
                    <li>"Monero (XMR) provides native privacy features including ring signatures, stealth addresses, and confidential transactions"</li>
                    <li>"Peer-to-peer architecture eliminates centralized data collection points"</li>
                </ul>
                <h3>"Third-Party Services"</h3>
                <p>
                    "This website does not integrate with third-party analytics, advertising, or tracking services. No external scripts are loaded that could collect user data."
                </p>
                <h3>"Your Rights"</h3>
                <p>
                    "Since no personal data is collected, there is no personal data to request, modify, or delete. If you believe we have collected your personal data in error, please contact us using the information provided in the Contact section below."
                </p>
            </div>
        </section>
    }
}

/// Terms of Use section
#[allow(non_snake_case)]
fn TermsOfUseSection() -> impl IntoView {
    view! {
        <section id="terms" class="section legal-section">
            <h2>"Terms of Use"</h2>
            <div class="legal-content">
                <p class="last-updated">"Last updated: October 2026"</p>
                <h3>"1. Nature of the Service"</h3>
                <p>
                    "The OpenDex website is an informational platform only. It does not accept orders, route trades, provide a trading interface, or act as a financial service provider. The website provides information about the OpenDex project, software downloads, and documentation."
                </p>
                <h3>"2. Software License"</h3>
                <p>
                    "OpenDex software is provided under the MIT and Apache-2.0 licenses. By using the software, you agree to the terms of these licenses. The software is provided 'as is' without warranty of any kind."
                </p>
                <h3>"3. User Responsibilities"</h3>
                <ul class="legal-list">
                    <li>"You are responsible for ensuring your use of OpenDex complies with applicable laws in your jurisdiction."</li>
                    <li>"You are responsible for securing your private keys and digital assets."</li>
                    <li>"You accept full responsibility for your trading decisions and associated risks."</li>
                    <li>"You will not use OpenDex for illegal activities or to circumvent applicable laws."</li>
                </ul>
                <h3>"4. No Financial Service"</h3>
                <p>
                    "OpenDex is not a financial service provider, exchange, broker, or custodian. The project does not hold, control, or have access to user funds. All transactions occur directly between P2P peers without intermediation."
                </p>
                <h3>"5. Limitation of Liability"</h3>
                <p>
                    "To the maximum extent permitted by law, the OpenDex project and its contributors shall not be liable for any direct, indirect, incidental, special, consequential, or punitive damages arising from your use of the software, including but not limited to loss of funds, data, or profits."
                </p>
                <h3>"6. Modifications"</h3>
                <p>
                    "The OpenDex project reserves the right to modify these terms at any time. Continued use of the website or software after modifications constitutes acceptance of the updated terms."
                </p>
            </div>
        </section>
    }
}

/// Contact section
#[allow(non_snake_case)]
fn ContactSection() -> impl IntoView {
    view! {
        <section id="contact" class="section legal-section">
            <h2>"Contact"</h2>
            <div class="legal-content">
                <p>
                    "For questions, support, or to report security vulnerabilities, please contact the OpenDex project team:"
                </p>
                <div class="contact-cards">
                    <div class="contact-card">
                        <h3>"📧 General Inquiries"</h3>
                        <p>"For general questions about OpenDex, the project, or the software."</p>
                        <a class="contact-link" href="mailto:contact@opendex.org">"contact@opendex.org"</a>
                    </div>
                    <div class="contact-card">
                        <h3>"🔒 Security Vulnerabilities"</h3>
                        <p>"To report security issues responsibly. Please do not disclose vulnerabilities publicly."</p>
                        <a class="contact-link" href="mailto:security@opendex.org">"security@opendex.org"</a>
                    </div>
                    <div class="contact-card">
                        <h3>"📋 Source Code & Documentation"</h3>
                        <p>"Browse the complete source code, audit reports, and regulatory compliance documentation."</p>
                        <a class="contact-link" href="/source#audit/">"View Source Browser"</a>
                    </div>
                </div>
                <p class="contact-note">
                    "For regulatory compliance documentation, see the source browser at"
                </p>
            </div>
        </section>
    }
}

/// Footer with copyright and links (used by both home and legal pages)
#[allow(non_snake_case)]
fn FooterSection() -> impl IntoView {
    view! {
        <footer class="footer">
            <p>"© 2026 OpenDex Project. Licensed under MIT OR Apache-2.0."</p>
            <p class="footer-links">
                <a href="/">"Home"</a>
                " · "
                <a href="/#about">"About"</a>
                " · "
                <a href="/#features">"Features"</a>
                " · "
                <a href="/#download">"Download"</a>
                " · "
                <a href="/#source">"Source"</a>
                " · "
                <a href="/legal">"Legal"</a>
                " · "
                <a href="/legal#privacy">"Privacy"</a>
                " · "
                <a href="/legal#terms">"Terms"</a>
                " · "
                <a href="/legal#contact">"Contact"</a>
            </p>
        </footer>
    }
}
