use leptos::prelude::*;
use leptos_meta::*;
use leptos_router::components::{Router, Routes, Route};
use leptos_router::path;

mod source_browser;
use source_browser::SourceBrowser;
mod legal;
use legal::LegalPage;

#[allow(non_snake_case)]
pub fn shell() -> impl IntoView {
    view! {
        <Title text="OpenDex — P2P Decentralized Exchange" />
        <Stylesheet href="/style.css" />
        <Meta name="description" content="OpenDex is a peer-to-peer decentralized exchange for trustless Bitcoin and Monero atomic swaps." />
        <Router>
            <Routes fallback=HomePage>
                <Route path=path!("/") view=HomePage />
                <Route path=path!("/source") view=SourceBrowserPage />
                <Route path=path!("/legal") view=LegalPage />
            </Routes>
        </Router>
    }
}

/// Home page with all informational sections
#[allow(non_snake_case)]
fn HomePage() -> impl IntoView {
    view! {
        <div class="container">
            <HeaderSection />
            <AboutSection />
            <FeaturesSection />
            <DownloadSection />
            <SourceSection />
            <FooterSection />
        </div>
    }
}

/// Source browser page wrapper
#[allow(non_snake_case)]
fn SourceBrowserPage() -> impl IntoView {
    view! {
        <SourceBrowser />
    }
}

/// Hero banner at the top of the page
#[allow(non_snake_case)]
fn HeaderSection() -> impl IntoView {
    view! {
        <section class="hero">
            <h1>"OpenDex"</h1>
            <p class="tagline">
                "Peer-to-Peer Decentralized Exchange for Trustless Bitcoin & Monero Swaps"
            </p>
            <div class="hero-buttons">
                <a class="btn btn-primary" href="#download">
                    "Download the App"
                </a>
                <a class="btn btn-secondary" href="#source">
                    "View Source Code"
                </a>
            </div>
        </section>
    }
}

/// About section explaining what OpenDex is
#[allow(non_snake_case)]
fn AboutSection() -> impl IntoView {
    view! {
        <section id="about" class="section">
            <h2>"What is OpenDex?"</h2>
            <div class="about-content">
                <p>
                    "OpenDex is an open-source, peer-to-peer decentralized exchange that enables trustless cross-chain swaps between Bitcoin (BTC) and Monero (XMR) without relying on any central authority or trusted third party."
                </p>
                <p>
                    "Built entirely in Rust using the libp2p networking stack, OpenDex uses GossipSub for order book synchronization, Kademlia DHT for peer discovery, and Noise protocol for encrypted communications. Trades are executed via atomic swaps using adaptor signatures, ensuring both parties receive their assets or neither does."
                </p>
                <p>
                    "Every node maintains a replicated view of orders, trades, balances, and trade state. The architecture is designed around deterministic trade identity, idempotent state application, authenticated actors, and conservative snapshot synchronization."
                </p>
            </div>
        </section>
    }
}

/// Features section highlighting key capabilities
#[allow(non_snake_case)]
fn FeaturesSection() -> impl IntoView {
    let features = vec![
        ("Peer-to-Peer Network", "No central server — every node is equal. Connect directly to other traders via libp2p with encrypted, authenticated communication."),
        ("Atomic Swaps", "Trustless BTC ↔ XMR swaps using adaptor signatures. Your funds are never at risk: either both parties receive their assets, or the swap is refunded."),
        ("Decentralized Order Book", "Orders are broadcast across the P2P network using GossipSub. Every node maintains its own copy of the order book — no central matching engine."),
        ("Privacy-First", "Monero-native privacy. No KYC, no central logs, no tracking. Your trading activity stays between you and the network."),
        ("Open Source", "Fully open-source codebase under MIT/Apache-2.0 licenses. Audit all components, run your own node, verify every transaction."),
        ("Cross-Platform", "Native desktop app for Linux, macOS, and Windows. Run your own P2P node and trade from anywhere."),
    ];

    view! {
        <section id="features" class="section">
            <h2>"Key Features"</h2>
            <div class="features-grid">
                {features.into_iter().map(|(title, desc)| {
                    view! {
                        <div class="feature-card">
                            <h3>{title}</h3>
                            <p>{desc}</p>
                        </div>
                    }
                }).collect_view()}
            </div>
        </section>
    }
}

/// Download section with links to native desktop apps
#[allow(non_snake_case)]
fn DownloadSection() -> impl IntoView {
    view! {
        <section id="download" class="section section-download">
            <h2>"Download OpenDex"</h2>
            <p class="section-subtitle">
                "Run your own P2P node and start trading. Choose your platform below."
            </p>
            <div class="download-cards">
                <div class="download-card">
                    <div class="platform-icon">"🐧"</div>
                    <h3>"Linux"</h3>
                    <p>"x86_64 AppImage & .deb package"</p>
                    <a class="btn btn-download" href="/releases/opendex-linux-x86_64.AppImage" download>
                        "Download AppImage"
                    </a>
                </div>
                <div class="download-card">
                    <div class="platform-icon">"🍎"</div>
                    <h3>"macOS"</h3>
                    <p>"Universal binary (Intel + Apple Silicon)"</p>
                    <a class="btn btn-download" href="/releases/opendex-macos-universal.dmg" download>
                        "Download .dmg"
                    </a>
                </div>
                <div class="download-card">
                    <div class="platform-icon">"🪟"</div>
                    <h3>"Windows"</h3>
                    <p>"x86_64 installer"</p>
                    <a class="btn btn-download" href="/releases/opendex-windows-x86_64.exe" download>
                        "Download .exe"
                    </a>
                </div>
            </div>
            <p class="download-note">
                "Latest release: v0.1.0 — "
                <a href="#source">"Build from source"</a>
                " for development versions."
            </p>
        </section>
    }
}

/// Source code and community section
#[allow(non_snake_case)]
fn SourceSection() -> impl IntoView {
    view! {
        <section id="source" class="section">
            <h2>"Source Code & Community"</h2>
            <div class="source-content">
                <p>
                    "OpenDex is fully open source. Browse the complete source codebase directly on this portal, or explore our project resources."
                </p>
                <div class="source-links">
                    <a class="source-card" href="/source">
                        <h3>"📦 Source Code Browser"</h3>
                        <p>"Browse all source files, audit docs, and architecture specs — hosted right here."</p>
                    </a>
                    <a class="source-card" href="#download">
                        <h3>"🚀 Downloads"</h3>
                        <p>"Get the latest native desktop app for your platform."</p>
                    </a>
                    <a class="source-card" href="/source#audit/">
                        <h3>"📋 Audit Reports"</h3>
                        <p>"Review security audits, architecture decisions, and compliance docs."</p>
                    </a>
                </div>
            </div>
        </section>
    }
}

/// Footer with copyright and links
#[allow(non_snake_case)]
fn FooterSection() -> impl IntoView {
    view! {
        <footer class="footer">
            <p>"© 2026 OpenDex Project. Licensed under MIT OR Apache-2.0."</p>
            <p class="footer-links">
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
