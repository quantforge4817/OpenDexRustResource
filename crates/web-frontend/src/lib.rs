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
            <ManifestosSection />
            <FeaturesSection />
            <DownloadSection />
            <SourceSection />
            <ContactSection />
            <FAQSection />
            <BlogSection />
            <SecurityAuditSection />
            <BugBountySection />
            <TutorialsSection />
            <DonationsSection />
            <DonationScripts />
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

/// About section explaining who, what, why, when, and how
#[allow(non_snake_case)]
fn AboutSection() -> impl IntoView {
    view! {
        <section id="about" class="section">
            <h2>"About OpenDex"</h2>
            <div class="about-content">

                <div class="about-block">
                    <h3>"Who We Are"</h3>
                    <p>
                        "OpenDex is a community-driven, open-source project built by people who believe financial sovereignty should be accessible to everyone. Developers, systems programmers, and privacy advocates come together around a shared goal: to build a decentralized exchange that empowers users to retain control of their assets throughout the trading process."
                    </p>
                    <p>
                        "OpenDex is not built around a single company or central authority. It is shaped by a distributed community of contributors who write code, review security, run nodes, and collaborate to strengthen the network. Together, we are working to build an open financial ecosystem that no single entity can unilaterally control or shut down."
                    </p>
                    <p>
                        "We believe financial freedom should not depend on who you are, where you live, or which centralized platform you trust. OpenDex exists to help make that freedom a practical reality."
                    </p>
                </div>

                <div class="about-block">
                    <h3>"What We Are Doing"</h3>
                    <p>
                        "We are building a peer-to-peer decentralized exchange that enables trustless cross-chain swaps between Bitcoin (BTC) and Monero (XMR). OpenDex is the middle ground where traders meet — we hold no assets and control no assets. Your assets never leave your wallet: they move directly between you and your counterparty, guaranteed by cryptography."
                    </p>
                    <p>
                        "Our technology stack includes a P2P network built on libp2p with GossipSub for order book synchronization, Kademlia DHT for peer discovery, and Noise protocol for encrypted communications. Trades execute via atomic swaps using adaptor signatures, guaranteeing that either both parties receive their assets or the swap is fully refunded."
                    </p>
                    <p>
                        "The project spans 20+ Rust crates organized into three milestones: atomic swap foundations, persistence and recovery systems, and P2P transport with peer discovery. A native desktop GUI and a static web portal provide user-facing access to the network."
                    </p>
                </div>

                <div class="about-block">
                    <h3>"Why We Are Doing It"</h3>
                    <p>
                        "Centralized exchanges have repeatedly exposed the risks of placing financial control in the hands of a single entity. Users deposit their assets into custodial accounts, trusting the exchange not to fail, freeze their assets, or misuse their funds. When an exchange collapses, the consequences can be devastating, leaving ordinary people facing significant financial losses."
                    </p>
                    <p>
                        "We believe cryptocurrency's promise is incomplete without the freedom to retain control of your assets throughout the trading process. OpenDex exists to bridge the gap between holding Bitcoin or Monero in your own wallet and exchanging them without surrendering control to a centralized intermediary."
                    </p>
                    <p>
                        "Privacy matters just as much. By combining Monero's native privacy features with a decentralized, peer-to-peer architecture, OpenDex aims to enable trading without mandatory KYC or dependence on a central authority to monitor trading activity. Your financial privacy should not come at the cost of access to the markets. Trading should not require you to entrust your financial activity to a centralized platform or leave it recorded in a central database vulnerable to exposure, surveillance, or subpoenas."
                    </p>
                    <p>
                        "OpenDex is built around a simple principle: you should be able to exchange assets without giving up control of them—or compromising your financial privacy in the process."
                    </p>
                </div>

                <div class="about-block">
                    <h3>"When We Started"</h3>
                    <p>
                        "OpenDex began on July 11, 2026, with the first commit laying the foundation for its peer-to-peer (P2P) network. Within days, the core networking layer was taking shape, bringing together libp2p peer discovery, Kademlia DHT, GossipSub for order book synchronization, and encrypted communication through the Noise protocol. By mid-July, distributed order matching and trade state synchronization were operational across the network."
                    </p>
                    <p>
                        "From August through September, the focus shifted to hardening the system and strengthening its security. Security audits identified issues in trade validation, order cancellation authentication, and state synchronization, which were subsequently addressed. Atomic persistence was introduced to protect against data loss following crashes, while the architecture evolved to support exchange extraction, authorized order submission, and deterministic trade identity."
                    </p>
                    <p>
                        "On October 14, 2026, OpenDex reached a major milestone with the release of v0.1.0-security-review—a frozen snapshot prepared for independent external security auditing. The snapshot covers 23 Rust crates and approximately 32,400 lines of code."
                    </p>
                    <p>
                        "Just one day later, v0.1.0-rc1 followed, with 425 passing tests, 133 penetration tests, 12 fuzz targets, and 24 Kani formal verification proofs. These milestones reflect a continued focus on testing, verification, and security as the project matures."
                    </p>
                    <p>
                        "OpenDex remains under active development, with security review and improvement continuing as the project moves forward."
                    </p>
                </div>

                <div class="about-block">
                    <h3>"How It Works"</h3>
                    <p>
                        "Getting started with OpenDex is simple: install the desktop app, start your node, and connect to the peer-to-peer (P2P) network. Your node discovers peers through Kademlia DHT, synchronizes order books using GossipSub, and communicates with other nodes over encrypted Noise channels."
                    </p>
                    <p>
                        "To trade, you create an order specifying the asset, amount, and price. Your order propagates across the network, allowing other participants to discover and accept it. Once a counterparty accepts the order, the atomic swap protocol begins, using adaptor signatures to help enforce the agreed exchange conditions."
                    </p>
                    <p>
                        "The swap coordinates the exchange between the Bitcoin and Monero blockchains without requiring OpenDex to take custody of your assets. OpenDex provides the infrastructure that allows peers to discover orders, negotiate trades, and exchange assets directly between their wallets. Discrete logarithm equality (DLEQ) proofs link the relevant cryptographic key material across the two blockchains, helping the protocol coordinate the exchange securely."
                    </p>
                    <p>
                        "The security of an atomic swap depends on the complete protocol, not on any single cryptographic mechanism. Adaptor signatures, blockchain confirmation requirements, timeout conditions, and deterministic recovery logic work together to govern how a swap proceeds and how assets can be reclaimed if it fails. OpenDex requires six Bitcoin confirmations and ten Monero confirmations before proceeding to redemption."
                    </p>
                    <p>
                        "If a swap stalls because a counterparty disappears, a timeout expires, or blockchain conditions change, the recovery mechanism evaluates the on-chain state and determines the appropriate redemption or refund action. These safeguards are designed to protect both parties and allow them to recover their assets when a swap cannot be completed, subject to the protocol's conditions and the relevant blockchain rules."
                    </p>
                    <p>
                        "No custodial escrow. No centralized intermediary. Just peer-to-peer trading built around cryptographic verification and automated recovery."
                    </p>
                    <p>
                        "Every node maintains its own replicated state, including orders, trades, balances, and trade history. Balances distinguish between available, locked, and escrowed amounts for each trader, with state persisted to disk and synchronized across the network. Deterministic trade identification, idempotent state updates, and authenticated actors help maintain consistent state across nodes, even during network partitions or node failures."
                    </p>
                </div>

            </div>
        </section>
    }
}

/// Manifestos and inspiration section with quotes and download links
#[allow(non_snake_case)]
fn ManifestosSection() -> impl IntoView {
    view! {
        <section id="manifestos" class="section">
            <h2>"Our Manifestos"</h2>
            <p class="section-subtitle">
                "Three principles guide everything we build: privacy, sovereignty, and decentralization. These are more than ideals; they are the foundations of an open financial ecosystem. The manifestos that follow reflect the ideas and convictions that continue to shape our vision for a more private, sovereign, and decentralized future."
            </p>
            <div class="manifestos-content">

                <div class="manifesto-block">
                    <h3>"🔐 A Cypherpunk's Manifesto"</h3>
                    <p class="manifesto-author">"by Eric Hughes, 1993"</p>
                    <blockquote class="manifesto-quote">
                        "Cypherpunks write code. We know that someone has to write software to defend privacy, and since we can't get privacy unless we all do, we're going to write it. We publish our code so that our fellow Cypherpunks may practice and play with it."
                    </blockquote>
                    <blockquote class="manifesto-quote">
                        "Privacy is necessary for an open society in the electronic age. Privacy is not secrecy. A private matter is something one doesn't want the whole world to know, but a secret matter is something one doesn't want anybody to know. Privacy is the power to selectively reveal oneself to the world."
                    </blockquote>
                    <p class="manifesto-connection">
                        "Privacy is our first principle. Eric Hughes understood that without cryptographic privacy, freedom is diminished and surveillance can become the price of convenience. OpenDex puts this principle into practice through peer-to-peer trading designed to minimize reliance on centralized intermediaries, identity checks, and centralized records of trading activity."
                    </p>
                    <p class="manifesto-connection">
                        "Your financial privacy is not an optional feature. It is a foundation of OpenDex. By making our code open source, we allow anyone to inspect our implementation, scrutinize our security, and verify our claims. We believe privacy should be protected by design, not granted at someone else's discretion."
                    </p>
                    <a class="btn btn-download" href="/docs/A Cypherpunk's Manifesto.odt" download>
                        "📄 Download Manifesto"
                    </a>
                </div>

                <div class="manifesto-block">
                    <h3>"₿ Bitcoin: A Peer-to-Peer Electronic Cash System"</h3>
                    <p class="manifesto-author">"by Satoshi Nakamoto, 2008"</p>
                    <blockquote class="manifesto-quote">
                        "A purely peer-to-peer version of electronic cash would allow online payments to be sent directly from one party to another without going through a financial institution."
                    </blockquote>
                    <blockquote class="manifesto-quote">
                        "What is needed is an electronic payment system based on cryptographic proof instead of trust, allowing any two willing parties to transact directly with each other without the need for a trusted third party."
                    </blockquote>
                    <p class="manifesto-connection">
                        "Decentralization is our second principle. Satoshi Nakamoto demonstrated how removing centralized intermediaries can give individuals greater control over their money and reduce reliance on trusted third parties. OpenDex extends this vision from payments to trading, enabling peer-to-peer exchanges between Bitcoin and Monero without relying on a centralized exchange to hold your assets or control your access to them."
                    </p>
                    <p class="manifesto-connection">
                        "By connecting directly with other participants, you retain control of your assets rather than entrusting them to a central platform. Cryptographic protocols underpin every BTC ↔ XMR atomic swap, reducing the need to trust a counterparty to act honestly. OpenDex is built around a simple principle: trading should be governed by verifiable rules, not the discretion of a centralized authority."
                    </p>
                    <a class="btn btn-download" href="/docs/bitcoinwhitepaper.pdf" download>
                        "📄 Download Whitepaper"
                    </a>
                </div>

                <div class="manifesto-block">
                    <h3>"🏴 The Crypto Anarchist Manifesto"</h3>
                    <p class="manifesto-author">"by Timothy C. May, 1988"</p>
                    <blockquote class="manifesto-quote">
                        "Computer technology is on the verge of providing the ability for individuals and groups to communicate and interact with each other in a totally anonymous manner. Two persons may exchange messages, conduct business, and negotiate electronic contracts without ever knowing the True Name, or legal identity, of the other."
                    </blockquote>
                    <blockquote class="manifesto-quote">
                        "Just as the technology of printing altered and reduced the power of medieval guilds and the social power structure, so too will cryptologic methods fundamentally alter the nature of corporations and of government interference in economic transactions."
                    </blockquote>
                    <p class="manifesto-connection">
                        "Sovereignty is our third principle. Timothy C. May envisioned a future in which cryptography would shift power away from institutions and back to individuals. OpenDex carries this vision into peer-to-peer trading, giving individuals greater control over how they exchange their assets."
                    </p>
                    <p class="manifesto-connection">
                        "With OpenDex, you retain control of your assets throughout the trading process, without entrusting them to a centralized exchange. Your private keys remain yours, and your ability to trade is not dependent on a central platform's permission. By combining self-custody, cryptography, and decentralized infrastructure, OpenDex works toward a future where financial sovereignty belongs to the individual."
                    </p>
                    <p class="manifesto-connection">
                        "You control your keys. You choose your trades. Your financial freedom should be yours to protect."
                    </p>
                    <a class="btn btn-download" href="/docs/timothy-c-may-crypto-anarchist-manifesto.pdf" download>
                        "📄 Download Manifesto"
                    </a>
                </div>

                <div class="manifesto-conclusion">
                    <h3>"Why These Principles Matter"</h3>
                    <p>
                        "Privacy, sovereignty, and decentralization are not abstract ideals. They are essential safeguards against the concentration of financial power."
                    </p>
                    <p>
                        "Centralized exchanges have repeatedly exposed the risks of putting trust in a single entity. They can freeze accounts, fail under pressure, and surrender control of users' assets to external authorities. OpenDex is founded on the principle that financial freedom depends on giving individuals control over their assets and reducing reliance on centralized intermediaries. By bringing privacy, sovereignty, and decentralization together, we aim to build an open financial ecosystem that no single entity can unilaterally control, censor, or shut down."
                    </p>
                    <p>
                        "These manifestos are not relics of the past. They are a call to action—one we answer with every line of code we write, every security audit we conduct, and every node that strengthens our network."
                    </p>
                    <p>
                        "We invite you to read these manifestos, understand the principles behind them, and join us in building a more decentralized financial future."
                    </p>
                </div>

            </div>
        </section>
    }
}

/// Features section highlighting key capabilities
#[allow(non_snake_case)]
fn FeaturesSection() -> impl IntoView {
    let features = vec![
        ("Peer-to-Peer Network", "No central server — every node is equal. Connect directly to other traders via libp2p with encrypted, authenticated communication."),
        ("Atomic Swaps", "BTC ↔ XMR swaps using DLEQ proofs — pure cryptography, no escrow. OpenDex holds and controls no assets: your assets move directly between your wallets, guaranteed by cross-chain cryptographic proofs."),
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

/// Download and installation section with OS-specific guides
#[allow(non_snake_case)]
fn DownloadSection() -> impl IntoView {
    view! {
        <section id="download" class="section section-download">
            <h2>"Download & Install OpenDex"</h2>
            <p class="section-subtitle">
                "Run your own P2P node and start trading. Follow the guide for your operating system below."
            </p>

            <div class="install-requirements">
                <h3>"System Requirements"</h3>
                <div class="requirements-grid">
                    <div class="requirement-item">
                        <strong>"Operating System"</strong>
                        <p>"Linux (Ubuntu 20.04+), macOS 12+ (Monterey), or Windows 10+ (64-bit)"</p>
                    </div>
                    <div class="requirement-item">
                        <strong>"Processor"</strong>
                        <p>"x86_64 or ARM64 (Apple Silicon). Minimum 2 cores recommended."</p>
                    </div>
                    <div class="requirement-item">
                        <strong>"Memory"</strong>
                        <p>"Minimum 2 GB RAM. 4 GB recommended for smooth operation."</p>
                    </div>
                    <div class="requirement-item">
                        <strong>"Storage"</strong>
                        <p>"500 MB for the application. Additional space for node data and trade history."</p>
                    </div>
                    <div class="requirement-item">
                        <strong>"Network"</strong>
                        <p>"Outbound TCP port 4001 for P2P peer connections. Inbound port optional for peer discovery."</p>
                    </div>
                    <div class="requirement-item">
                        <strong>"Display"</strong>
                        <p>"Minimum 1280×800 resolution. The app window opens at 1200×800."</p>
                    </div>
                </div>
            </div>

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

            <div class="install-guide" id="install-linux">
                <h3>"🐧 Linux Installation Guide"</h3>

                <div class="install-method">
                    <h4>"Method 1: AppImage (Recommended)"</h4>
                    <ol class="install-steps">
                        <li>
                            <strong>"Download the AppImage"</strong>
                            <p>"Click the Download AppImage button above, or run:"</p>
                            <code>"wget https://opendex.org/releases/opendex-linux-x86_64.AppImage -O ~/Downloads/opendex.AppImage"</code>
                        </li>
                        <li>
                            <strong>"Make it executable"</strong>
                            <code>"chmod +x ~/Downloads/opendex.AppImage"</code>
                        </li>
                        <li>
                            <strong>"Run the application"</strong>
                            <code>"~/Downloads/opendex.AppImage"</code>
                        </li>
                        <li>
                            <strong>"Optional: Install system-wide"</strong>
                            <p>"Move the AppImage to a permanent location:"</p>
                            <code>"sudo mv ~/Downloads/opendex.AppImage /usr/local/bin/opendex"</code>
                        </li>
                    </ol>
                </div>

                <div class="install-method">
                    <h4>"Method 2: .deb Package (Ubuntu/Debian)"</h4>
                    <ol class="install-steps">
                        <li>
                            <strong>"Download the .deb package"</strong>
                            <code>"wget https://opendex.org/releases/opendex-linux-x86_64.deb -O ~/Downloads/opendex.deb"</code>
                        </li>
                        <li>
                            <strong>"Install the package"</strong>
                            <code>"sudo dpkg -i ~/Downloads/opendex.deb"</code>
                        </li>
                        <li>
                            <strong>"Fix missing dependencies (if needed)"</strong>
                            <code>"sudo apt-get install -f"</code>
                        </li>
                        <li>
                            <strong>"Launch from your application menu"</strong>
                            <p>"Search for OpenDex in your applications, or run:"</p>
                            <code>"opendex"</code>
                        </li>
                    </ol>
                </div>

                <div class="install-callout install-callout-warning">
                    <strong>"⚠ Note for headless servers"</strong>
                    <p>"The desktop GUI requires a display server (X11 or Wayland). On headless machines, use SSH with X11 forwarding (ssh -X) or a virtual display (xvfb-run opendex)."</p>
                </div>
            </div>

            <div class="install-guide" id="install-macos">
                <h3>"🍎 macOS Installation Guide"</h3>

                <div class="install-method">
                    <h4>"Install from .dmg"</h4>
                    <ol class="install-steps">
                        <li>
                            <strong>"Download the .dmg file"</strong>
                            <p>"Click the Download .dmg button above, or run:"</p>
                            <code>"curl -L https://opendex.org/releases/opendex-macos-universal.dmg -o ~/Downloads/OpenDex.dmg"</code>
                        </li>
                        <li>
                            <strong>"Open the disk image"</strong>
                            <p>"Double-click OpenDex.dmg in your Downloads folder."</p>
                        </li>
                        <li>
                            <strong>"Drag to Applications"</strong>
                            <p>"Drag the OpenDex icon into the Applications folder in the mounted disk image window."</p>
                        </li>
                        <li>
                            <strong>"Handle Gatekeeper warning"</strong>
                            <p>"On first launch, macOS may block the app. Right-click the app in Applications, select Open, then confirm by clicking Open again."</p>
                        </li>
                        <li>
                            <strong>"Launch OpenDex"</strong>
                            <p>"Open the app from your Applications folder or via Spotlight search."</p>
                        </li>
                    </ol>
                </div>

                <div class="install-method">
                    <h4>"Install from Terminal (alternative)"</h4>
                    <ol class="install-steps">
                        <li>
                            <strong>"Download and mount the disk image"</strong>
                            <code>"curl -L https://opendex.org/releases/opendex-macos-universal.dmg -o ~/Downloads/OpenDex.dmg"</code>
                            <code>"hdiutil attach ~/Downloads/OpenDex.dmg"</code>
                        </li>
                        <li>
                            <strong>"Copy to Applications"</strong>
                            <code>"cp -R /Volumes/OpenDex/OpenDex.app /Applications/"</code>
                        </li>
                        <li>
                            <strong>"Eject the disk image"</strong>
                            <code>"hdiutil detach /Volumes/OpenDex"</code>
                        </li>
                    </ol>
                </div>

                <div class="install-callout install-callout-info">
                    <strong>"ℹ Apple Silicon support"</strong>
                    <p>"The universal binary runs natively on both Intel Macs and Apple Silicon (M1/M2/M3). No Rosetta translation is needed."</p>
                </div>
            </div>

            <div class="install-guide" id="install-windows">
                <h3>"🪟 Windows Installation Guide"</h3>

                <div class="install-method">
                    <h4>"Install from .exe"</h4>
                    <ol class="install-steps">
                        <li>
                            <strong>"Download the installer"</strong>
                            <p>"Click the Download .exe button above, or download it directly from the releases page."</p>
                        </li>
                        <li>
                            <strong>"Run the installer"</strong>
                            <p>"Double-click opendex-windows-x86_64.exe. If Windows SmartScreen appears, click More info, then Run anyway."</p>
                        </li>
                        <li>
                            <strong>"Follow the installation wizard"</strong>
                            <p>"Accept the license agreement, choose an installation directory (default is recommended), and click Install."</p>
                        </li>
                        <li>
                            <strong>"Launch OpenDex"</strong>
                            <p>"The installer will offer to launch the app. You can also find it in your Start Menu."</p>
                        </li>
                    </ol>
                </div>

                <div class="install-method">
                    <h4>"Install via PowerShell (alternative)"</h4>
                    <ol class="install-steps">
                        <li>
                            <strong>"Open PowerShell as Administrator"</strong>
                            <p>"Right-click the Start button and select Windows PowerShell (Admin) or Terminal (Admin)."</p>
                        </li>
                        <li>
                            <strong>"Download and run the installer"</strong>
                            <code>"Invoke-WebRequest -Uri https://opendex.org/releases/opendex-windows-x86_64.exe -OutFile $env:USERPROFILE\\Downloads\\opendex-setup.exe"</code>
                            <code>"Start-Process $env:USERPROFILE\\Downloads\\opendex-setup.exe -Wait"</code>
                        </li>
                    </ol>
                </div>

                <div class="install-callout install-callout-warning">
                    <strong>"⚠ Windows Defender SmartScreen"</strong>
                    <p>"Because OpenDex is a new, unsigned application, Windows SmartScreen may display a warning. This is expected for open-source software. Click More info and Run anyway to proceed."</p>
                </div>
            </div>

            <div class="install-guide" id="install-source">
                <h3>"🔨 Build from Source"</h3>
                <p class="install-intro">
                    "If you prefer to build OpenDex yourself, or want to run the latest development version, follow these steps."
                </p>

                <div class="install-method">
                    <h4>"Prerequisites"</h4>
                    <ol class="install-steps">
                        <li>
                            <strong>"Install Rust"</strong>
                            <p>"Follow the instructions at rustup.rs to install the Rust toolchain."</p>
                            <code>"curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"</code>
                        </li>
                        <li>
                            <strong>"Install system dependencies"</strong>
                            <p>"Linux (Ubuntu/Debian):"</p>
                            <code>"sudo apt-get install build-essential pkg-config libssl-dev libudev-dev"</code>
                            <p>"macOS (with Homebrew):"</p>
                            <code>"brew install openssl pkg-config"</code>
                            <p>"Windows (with winget):"</p>
                            <code>"winget install Microsoft.VisualStudio.Workload.VCTools OpenSSL"</code>
                        </li>
                    </ol>
                </div>

                <div class="install-method">
                    <h4>"Build and Run"</h4>
                    <ol class="install-steps">
                        <li>
                            <strong>"Clone the repository"</strong>
                            <code>"git clone https://github.com/OpenDex/OpenDexRustSource.git"</code>
                            <code>"cd OpenDexRustSource"</code>
                        </li>
                        <li>
                            <strong>"Build the desktop app"</strong>
                            <code>"cargo build --release -p desktop-gui"</code>
                            <p>"This may take several minutes on first build."</p>
                        </li>
                        <li>
                            <strong>"Run the application"</strong>
                            <code>"cargo run --release -p desktop-gui"</code>
                        </li>
                    </ol>
                </div>

                <div class="install-method">
                    <h4>"Command-Line Options"</h4>
                    <div class="cli-options">
                        <div class="cli-option">
                            <code>"--port 4001"</code>
                            <p>"Set the P2P node listening port (default: 4001)"</p>
                        </div>
                        <div class="cli-option">
                            <code>"--peer /ip4/.../p2p/PEER_ID"</code>
                            <p>"Connect to a specific peer on startup"</p>
                        </div>
                        <div class="cli-option">
                            <code>"--new-identity"</code>
                            <p>"Generate a fresh node identity (PeerID)"</p>
                        </div>
                        <div class="cli-option">
                            <code>"--monero-rpc http://127.0.0.1:18081"</code>
                            <p>"Connect Monero wallet RPC server"</p>
                        </div>
                        <div class="cli-option">
                            <code>"--btc-rpc http://127.0.0.1:8332"</code>
                            <p>"Connect Bitcoin wallet RPC server"</p>
                        </div>
                    </div>
                </div>
            </div>

            <div class="install-guide" id="install-walkthrough">
                <h3>"🚀 From Download to Your First Trade"</h3>
                <p class="install-intro">
                    "Once installed, here is the complete journey from first launch to your first trade."
                </p>

                <div class="walkthrough-steps">
                    <div class="walkthrough-step">
                        <div class="step-number">"1"</div>
                        <div class="step-content">
                            <h4>"Launch the App"</h4>
                            <p>"Open OpenDex. On first run, the app generates a unique node identity (PeerID) and starts a P2P node listening on port 4001. The window opens at 1200×800."</p>
                        </div>
                    </div>

                    <div class="walkthrough-step">
                        <div class="step-number">"2"</div>
                        <div class="step-content">
                            <h4>"Create Your Account"</h4>
                            <p>"The login screen appears. Click Register and fill in your username, password, and email address. You can optionally add your Bitcoin and Monero wallet addresses now, or add them later."</p>
                        </div>
                    </div>

                    <div class="walkthrough-step">
                        <div class="step-number">"3"</div>
                        <div class="step-content">
                            <h4>"Verify Your Email"</h4>
                            <p>"A 6-digit verification code is generated. Enter it on the verification screen to confirm your account. (During development, the code appears in the console logs.)"</p>
                        </div>
                    </div>

                    <div class="walkthrough-step">
                        <div class="step-number">"4"</div>
                        <div class="step-content">
                            <h4>"Log In"</h4>
                            <p>"Enter your username and password. On success, the main trading interface appears with six panels: status bar, order book, trade form, trade history, wallet panel, and peer management."</p>
                        </div>
                    </div>

                    <div class="walkthrough-step">
                        <div class="step-number">"5"</div>
                        <div class="step-content">
                            <h4>"Connect to a Peer"</h4>
                            <p>"Before trading, connect to at least one peer on the network. Enter a peer's multiaddress in the Peer Management panel and click Connect. The order book will populate with orders from the network."</p>
                        </div>
                    </div>

                    <div class="walkthrough-step">
                        <div class="step-number">"6"</div>
                        <div class="step-content">
                            <h4>"(Optional) Connect Your Wallets"</h4>
                            <p>"For full functionality, connect your Monero and Bitcoin wallet RPC servers using the command-line flags, or add your wallet addresses in your account settings."</p>
                        </div>
                    </div>

                    <div class="walkthrough-step">
                        <div class="step-number">"7"</div>
                        <div class="step-content">
                            <h4>"Place Your First Order"</h4>
                            <p>"In the Trade Form, select Buy or Sell, set a price (XMR per BTC) and amount, then click Place Order. Your order is broadcast to the P2P network."</p>
                        </div>
                    </div>

                    <div class="walkthrough-step">
                        <div class="step-number">"8"</div>
                        <div class="step-content">
                            <h4>"Trade Executes via Atomic Swap"</h4>
                            <p>"When a counterparty accepts your order, an atomic swap begins. Assets are locked using DLEQ proofs — pure cryptography, no escrow. OpenDex holds and controls no assets. Your assets move directly between your wallets."</p>
                        </div>
                    </div>
                </div>

                <div class="install-callout install-callout-info">
                    <strong>"🔒 Your Data Is Saved"</strong>
                    <p>"Your account, reputation, and trade history are stored locally in ~/.opendex/users.json and persist across app restarts. Close the app and return whenever you're ready—your saved data will be there when you reopen OpenDex."</p>
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

/// Contact and communications section
#[allow(non_snake_case)]
fn ContactSection() -> impl IntoView {
    view! {
        <section id="contact" class="section">
            <h2>"Contact & Communications"</h2>
            <p class="section-subtitle">
                "Have a question, bug report, feature request, or want to contribute? Reach out through the channel that best fits your needs."
            </p>
            <div class="contact-grid">

                <div class="contact-card">
                    <div class="contact-icon">"🐛"</div>
                    <h3>"Report a Bug"</h3>
                    <p>
                        "Encountered a bug or unexpected behavior? Help us improve OpenDex by reporting it on GitHub. Include as much detail as possible, such as steps to reproduce the issue, your operating system, and any relevant error messages or logs. Clear, detailed reports help us identify problems and make OpenDex more reliable and secure for everyone."
                    </p>
                    <a class="btn btn-contact" href="https://github.com/OpenDex/OpenDexRustSource/issues" target="_blank" rel="noopener">
                        "File an Issue"
                    </a>
                </div>

                <div class="contact-card">
                    <div class="contact-icon">"💡"</div>
                    <h3>"Feature Request"</h3>
                    <p>
                        "Have an idea for improving OpenDex? Share it on GitHub Discussions so the community can weigh in and help shape the project's direction."
                    </p>
                    <a class="btn btn-contact" href="https://github.com/OpenDex/OpenDexRustSource/discussions" target="_blank" rel="noopener">
                        "Start a Discussion"
                    </a>
                </div>

                <div class="contact-card">
                    <div class="contact-icon">"💬"</div>
                    <h3>"Community Chat"</h3>
                    <p>
                        "Join the OpenDex community on Matrix to ask questions, discuss development, connect with contributors, and share ideas. Whether you're a developer, privacy advocate, or simply curious about decentralized trading, you're welcome to join the conversation."
                    </p>
                    <a class="btn btn-contact" href="https://matrix.to/#/#opendex:opendex.org" target="_blank" rel="noopener">
                        "Join Matrix"
                    </a>
                </div>

                <div class="contact-card">
                    <div class="contact-icon">"📧"</div>
                    <h3>"General Inquiries"</h3>
                    <p>
                        "Have a question about OpenDex, a partnership proposal, a press inquiry, or something else? Get in touch with us by email. We're happy to hear from individuals, organizations, and anyone interested in our mission to build a more private, sovereign, and decentralized financial ecosystem."
                    </p>
                    <a class="btn btn-contact" href="mailto:contact@opendex.org">
                        "Send Email"
                    </a>
                </div>

                <div class="contact-card">
                    <div class="contact-icon">"🔐"</div>
                    <h3>"Security Reports"</h3>
                    <p>
                        "Discovered a potential security vulnerability in OpenDex? Please report it responsibly and give us an opportunity to investigate and address the issue before publicly disclosing the details. We take security seriously and appreciate the time and care researchers and community members dedicate to helping keep OpenDex secure."
                    </p>
                    <a class="btn btn-contact" href="mailto:security@opendex.org">
                        "Report Vulnerability"
                    </a>
                </div>

                <div class="contact-card">
                    <div class="contact-icon">"🤝"</div>
                    <h3>"Contribute"</h3>
                    <p>
                        "Want to help build OpenDex? Browse the source code, read the contributing guidelines, and submit pull requests. All contributions are welcome."
                    </p>
                    <a class="btn btn-contact" href="/source">
                        "Browse Source"
                    </a>
                </div>

            </div>

            <div class="contact-note">
                <p>
                    "We aim to respond within 48 hours. For urgent matters, Matrix is our most active communication channel."
                </p>
            </div>
        </section>
    }
}

/// Donation section for crypto contributions
#[allow(non_snake_case)]
fn DonationsSection() -> impl IntoView {
    view! {
        <section id="donate" class="section donations-section">
            <h2>"Support OpenDex"</h2>
            <p class="section-subtitle">
                "OpenDex is an open-source, community-driven project built on the principles of privacy, sovereignty, and decentralization. If you believe in our mission, consider supporting its development."
            </p>
            <p class="section-subtitle">
                "Your contributions help fund ongoing development, independent security audits, and the infrastructure that keeps the network growing. Every contribution helps strengthen the project, improve its security, and bring us closer to a more open and decentralized financial ecosystem."
            </p>
            <p class="section-subtitle">
                "Together, we can build a future where financial freedom is accessible to everyone."
            </p>

            <div class="donation-cards">
                <div class="donation-card">
                    <div class="donation-crypto-icon">"₿"</div>
                    <h3>"Bitcoin (BTC)"</h3>
                    <p class="donation-description">
                        "Support OpenDex by donating Bitcoin directly to our project wallet. All BTC contributions go toward core protocol development, independent security audits, and strengthening the security of the network."
                    </p>
                    <div class="wallet-address-box">
                        <code class="wallet-address">"bc1qopendex..."</code>
                        <button class="btn-copy" onclick="copyToClipboard('bc1qopendexplaceholderaddress1234567890abcdef', this)">
                            "📋 Copy"
                        </button>
                    </div>
                </div>

                <div class="donation-card">
                    <div class="donation-crypto-icon">"ɱ"</div>
                    <h3>"Monero (XMR)"</h3>
                    <p class="donation-description">
                        "Support OpenDex by donating Monero directly to our project wallet. All XMR contributions help fund privacy research, Monero integration, and ongoing development of secure, decentralized trading."
                    </p>
                    <div class="wallet-address-box">
                        <code class="wallet-address">"4..."</code>
                        <button class="btn-copy" onclick="copyToClipboard('4placeholdermoneroaddress1234567890abcdefghijklmnopqrstuvwxyz', this)">
                            "📋 Copy"
                        </button>
                    </div>
                </div>
            </div>

            <div class="donation-note">
                <p>
                    "🔒 Donations are non-refundable and go directly toward supporting OpenDex development, security, and infrastructure. Thank you for helping us build a more open, private, and decentralized financial ecosystem."
                </p>
                <p class="donation-transparency">
                    "Please note that Bitcoin transactions are publicly visible on the Bitcoin blockchain, while Monero transactions benefit from Monero's built-in privacy features. For large donations or sponsorship inquiries, contact us at "
                    <a href="mailto:contact@opendex.org">"contact@opendex.org"</a>
                    "."
                </p>
            </div>
        </section>
    }
}

/// Frequently Asked Questions section
#[allow(non_snake_case)]
fn FAQSection() -> impl IntoView {
    view! {
        <section id="faq" class="section">
            <h2>"Frequently Asked Questions"</h2>
            <p class="section-subtitle">
                "Common questions about OpenDex, atomic swaps, wallets, and more."
            </p>
            <div class="faq-list">

                <div class="faq-item">
                    <h3>"What is an atomic swap?"</h3>
                    <p>
                        "An atomic swap is a cryptographic protocol that allows two parties to exchange cryptocurrencies across different blockchains without trusting each other or a third party. Either both sides of the trade complete, or neither does — there is no partial execution. OpenDex uses adaptor signatures and DLEQ (Discrete Log Equality) proofs to enable trustless Bitcoin-to-Monero swaps."
                    </p>
                </div>

                <div class="faq-item">
                    <h3>"How do I connect a wallet?"</h3>
                    <p>
                        "OpenDex does not require you to deposit assets into a custodial wallet. Instead, you connect your existing Bitcoin and Monero wallets via command-line flags when launching the app. For Bitcoin, use --btc-rpc with your node's RPC URL. For Monero, use --monero-rpc with your wallet RPC URL. Your assets remain in your wallet at all times."
                    </p>
                </div>

                <div class="faq-item">
                    <h3>"What happens if a trade fails?"</h3>
                    <p>
                        "If a swap cannot be completed — for example, if a counterparty disappears or a timeout expires — the protocol's deterministic recovery engine evaluates the on-chain state and initiates the appropriate refund. Your assets are returned to your wallet, subject to blockchain confirmation requirements (six Bitcoin confirmations, ten Monero confirmations). No assets are lost."
                    </p>
                </div>

                <div class="faq-item">
                    <h3>"Is my data safe?"</h3>
                    <p>
                        "Yes. OpenDex stores your account data, reputation, and trade history locally on your machine in ~/.opendex/users.json. No personal data is sent to any central server. All P2P communication is encrypted using the Noise protocol. OpenDex holds and controls no assets — your funds never leave your wallet."
                    </p>
                </div>

                <div class="faq-item">
                    <h3>"What is the reputation system?"</h3>
                    <p>
                        "Every user account tracks trading activity: orders placed, trades completed, disputes, and written reviews from counterparties. This data is used to calculate a trust score (0–100) and assign a reputation tier (Newcomer, Trader, Trusted, Verified, Elite). You can view a counterparty's reputation before accepting their order to assess trading risk."
                    </p>
                </div>

                <div class="faq-item">
                    <h3>"How do I install OpenDex?"</h3>
                    <p>
                        "OpenDex is available for Linux, macOS, and Windows. Visit the "</p>
                    <p>
                        <a href="#download">"Download section"</a>
                        " for step-by-step installation guides for each operating system, including AppImage, .deb, .dmg, and .exe formats. You can also build from source using Rust and Cargo."
                    </p>
                </div>

                <div class="faq-item">
                    <h3>"Do I need to run a full node?"</h3>
                    <p>
                        "Yes. OpenDex is a peer-to-peer application — each user runs a local node that embeds the full P2P networking stack. This means there is no central server to rely on. Your node discovers peers, synchronizes order books, and participates in the network. No separate full Bitcoin or Monero node is required for the P2P layer, though you will need wallet RPC access to trade."
                    </p>
                </div>

                <div class="faq-item">
                    <h3>"What cryptocurrencies does OpenDex support?"</h3>
                    <p>
                        "Currently, OpenDex supports atomic swaps between Bitcoin (BTC) and Monero (XMR). Additional trading pairs may be added in the future as the protocol matures."
                    </p>
                </div>

                <div class="faq-item">
                    <h3>"Is OpenDex open source?"</h3>
                    <p>
                        "Yes. OpenDex is fully open source, licensed under MIT OR Apache-2.0. The complete source code is available on GitHub. Anyone can inspect the code, review the security audits, run the tests, and contribute to the project."
                    </p>
                </div>

                <div class="faq-item">
                    <h3>"How do I report a security vulnerability?"</h3>
                    <p>
                        "If you discover a security vulnerability, please report it responsibly at "</p>
                    <p>
                        <a href="mailto:security@opendex.org">"security@opendex.org"</a>
                        " so we can investigate and address the issue before public disclosure. We appreciate responsible disclosure and encourage security researchers to help keep OpenDex safe."
                    </p>
                </div>

            </div>
        </section>
    }
}

/// Blog / News section with development updates
#[allow(non_snake_case)]
fn BlogSection() -> impl IntoView {
    view! {
        <section id="blog" class="section blog-section">
            <h2>"Development Updates"</h2>
            <p class="section-subtitle">
                "Follow the latest news, milestones, and educational content from the OpenDex project."
            </p>
            <div class="blog-list">

                <div class="blog-card">
                    <div class="blog-date">"October 15, 2026"</div>
                    <h3>"v0.1.0-rc1 Released"</h3>
                    <p>
                        "OpenDex reaches release candidate status with 425 passing tests, 133 penetration tests, 12 fuzz targets, and 24 Kani formal verification proofs. This milestone represents months of security hardening, architectural refinement, and rigorous testing."
                    </p>
                    <span class="blog-tag">"Release"</span>
                </div>

                <div class="blog-card">
                    <div class="blog-date">"October 14, 2026"</div>
                    <h3>"Security Review Snapshot"</h3>
                    <p>
                        "A frozen code snapshot (v0.1.0-security-review) covering 23 Rust crates and approximately 32,400 lines of code has been prepared for independent external security auditing."
                    </p>
                    <span class="blog-tag">"Security"</span>
                </div>

                <div class="blog-card">
                    <div class="blog-date">"September 2026"</div>
                    <h3>"Security Hardening Complete"</h3>
                    <p>
                        "Addressed critical findings from internal security audits, including trade validation, order cancellation authentication, and state synchronization. Atomic persistence implemented to protect against data loss after crashes."
                    </p>
                    <span class="blog-tag">"Security"</span>
                </div>

                <div class="blog-card">
                    <div class="blog-date">"August 2026"</div>
                    <h3>"Architecture Evolution"</h3>
                    <p>
                        "The project architecture evolved to support exchange extraction, authorized order submission, and deterministic trade identity. The P2P networking layer continues to mature with improved peer discovery and order book synchronization."
                    </p>
                    <span class="blog-tag">"Development"</span>
                </div>

                <div class="blog-card">
                    <div class="blog-date">"July 2026"</div>
                    <h3>"Project Launch"</h3>
                    <p>
                        "OpenDex begins with the first commit, laying the foundation for a P2P network built on libp2p, Kademlia DHT, GossipSub, and the Noise protocol. Distributed order matching and trade state synchronization become operational within weeks."
                    </p>
                    <span class="blog-tag">"Milestone"</span>
                </div>

            </div>
        </section>
    }
}

/// Security Audit Badge section
#[allow(non_snake_case)]
fn SecurityAuditSection() -> impl IntoView {
    view! {
        <section id="security" class="section security-audit-section">
            <h2>"Security & Verification"</h2>
            <p class="section-subtitle">
                "OpenDex is built with security as a core principle. Every line of code is tested, audited, and formally verified."
            </p>
            <div class="security-badges">

                <div class="security-badge">
                    <div class="badge-icon">"✅"</div>
                    <div class="badge-number">"425"</div>
                    <div class="badge-label">"Passing Tests"</div>
                </div>

                <div class="security-badge">
                    <div class="badge-icon">"🛡️"</div>
                    <div class="badge-number">"133"</div>
                    <div class="badge-label">"Penetration Tests"</div>
                </div>

                <div class="security-badge">
                    <div class="badge-icon">"🔬"</div>
                    <div class="badge-number">"12"</div>
                    <div class="badge-label">"Fuzz Targets"</div>
                </div>

                <div class="security-badge">
                    <div class="badge-icon">"🔐"</div>
                    <div class="badge-number">"24"</div>
                    <div class="badge-label">"Kani Proofs"</div>
                </div>

                <div class="security-badge">
                    <div class="badge-icon">"📦"</div>
                    <div class="badge-number">"23"</div>
                    <div class="badge-label">"Rust Crates"</div>
                </div>

                <div class="security-badge">
                    <div class="badge-icon">"📝"</div>
                    <div class="badge-number">"~32,400"</div>
                    <div class="badge-label">"Lines of Code"</div>
                </div>

            </div>

            <div class="security-status">
                <div class="status-item">
                    <span class="status-dot status-pass"></span>
                    <span>"cargo test — All passing"</span>
                </div>
                <div class="status-item">
                    <span class="status-dot status-pass"></span>
                    <span>"cargo clippy — No warnings"</span>
                </div>
                <div class="status-item">
                    <span class="status-dot status-pass"></span>
                    <span>"cargo-deny — No forbidden dependencies"</span>
                </div>
                <div class="status-item">
                    <span class="status-dot status-pass"></span>
                    <span>"cargo-audit — No known vulnerabilities"</span>
                </div>
                <div class="status-item">
                    <span class="status-dot status-review"></span>
                    <span>"External security audit — In progress"</span>
                </div>
            </div>

            <div class="security-note">
                <p>
                    "🔒 OpenDex underwent internal security audits in August–September 2026, identifying and resolving critical findings in trade validation, order cancellation authentication, and state synchronization. A frozen snapshot (v0.1.0-security-review) has been prepared for independent external auditing."
                </p>
            </div>
        </section>
    }
}

/// Bug Bounty Program section
#[allow(non_snake_case)]
fn BugBountySection() -> impl IntoView {
    view! {
        <section id="bug-bounty" class="section bug-bounty-section">
            <h2>"Bug Bounty Program"</h2>
            <p class="section-subtitle">
                "Help us make OpenDex more secure. We reward responsible disclosure of vulnerabilities."
            </p>

            <div class="bug-bounty-content">

                <div class="bounty-block">
                    <h3>"How to Report"</h3>
                    <p>
                        "If you discover a security vulnerability in OpenDex, please report it responsibly by emailing "
                        <a href="mailto:security@opendex.org">"security@opendex.org"</a>
                        ". Include a clear description of the vulnerability, steps to reproduce, and any potential impact. We aim to respond within 48 hours."
                    </p>
                </div>

                <div class="bounty-block">
                    <h3>"Scope"</h3>
                    <p>"In scope for rewards:"</p>
                    <ul>
                        <li>"Remote code execution vulnerabilities"</li>
                        <li>"Cryptographic protocol flaws in atomic swaps"</li>
                        <li>"Authentication bypass in user accounts"</li>
                        <li>"P2P network protocol vulnerabilities"</li>
                        <li>"Denial of service attacks against the node"</li>
                        <li>"Data corruption in persistence layer"</li>
                        <li>"Privilege escalation in trade execution"</li>
                    </ul>
                    <p>"Out of scope:"</p>
                    <ul>
                        <li>"Vulnerabilities in third-party dependencies (report to their maintainers)"</li>
                        <li>"Social engineering attacks"</li>
                        <li>"Issues requiring physical access to a user's machine"</li>
                    </ul>
                </div>

                <div class="bounty-block">
                    <h3>"Rewards"</h3>
                    <p>
                        "Rewards are based on the severity and impact of the vulnerability:"</p>
                    <ul>
                        <li>"<strong>"Critical"</strong> — Remote code execution, cryptographic break: up to 1 BTC or 500 XMR"</li>
                        <li>"<strong>"High"</strong> — Authentication bypass, data corruption: up to 0.5 BTC or 250 XMR"</li>
                        <li>"<strong>"Medium"</strong> — Denial of service, privilege escalation: up to 0.1 BTC or 50 XMR"</li>
                        <li>"<strong>"Low"</strong> — Minor issues with limited impact: up to 0.01 BTC or 10 XMR"</li>
                    </ul>
                    <p>
                        "All rewards are paid in cryptocurrency to your provided wallet address. We also publicly acknowledge contributors (with permission) in our release notes."
                    </p>
                </div>

                <div class="bounty-block">
                    <h3>"Responsible Disclosure"</h3>
                    <p>
                        "We ask that you do not publicly disclose vulnerabilities until we have had a reasonable opportunity to investigate and release a fix. We will work with you to coordinate a responsible disclosure timeline. We do not take legal action against researchers who follow responsible disclosure practices."
                    </p>
                </div>

            </div>
        </section>
    }
}

/// Video Tutorials section
#[allow(non_snake_case)]
fn TutorialsSection() -> impl IntoView {
    view! {
        <section id="tutorials" class="section tutorials-section">
            <h2>"Video Tutorials"</h2>
            <p class="section-subtitle">
                "Learn how to use OpenDex with step-by-step video guides. New tutorials are added as features are released."
            </p>
            <div class="tutorials-grid">

                <div class="tutorial-card">
                    <div class="tutorial-thumbnail">
                        <div class="play-button">"▶"</div>
                        <span class="tutorial-duration">"5:30"</span>
                    </div>
                    <h3>"Getting Started with OpenDex"</h3>
                    <p>
                        "A complete walkthrough of downloading, installing, and launching OpenDex for the first time. Learn about system requirements, account creation, and connecting to the P2P network."
                    </p>
                    <span class="tutorial-tag">"Beginner"</span>
                </div>

                <div class="tutorial-card">
                    <div class="tutorial-thumbnail">
                        <div class="play-button">"▶"</div>
                        <span class="tutorial-duration">"8:15"</span>
                    </div>
                    <h3>"Your First Trade"</h3>
                    <p>
                        "Step-by-step guide to placing your first BTC-to-XMR atomic swap. Covers order creation, counterparty selection, reputation checking, and monitoring the swap progress."
                    </p>
                    <span class="tutorial-tag">"Beginner"</span>
                </div>

                <div class="tutorial-card">
                    <div class="tutorial-thumbnail">
                        <div class="play-button">"▶"</div>
                        <span class="tutorial-duration">"6:45"</span>
                    </div>
                    <h3>"Connecting Wallets"</h3>
                    <p>
                        "How to connect your Bitcoin and Monero wallets to OpenDex using RPC. Covers wallet configuration, command-line flags, and verifying your connection."
                    </p>
                    <span class="tutorial-tag">"Intermediate"</span>
                </div>

                <div class="tutorial-card">
                    <div class="tutorial-thumbnail">
                        <div class="play-button">"▶"</div>
                        <span class="tutorial-duration">"7:20"</span>
                    </div>
                    <h3>"Understanding Reputation"</h3>
                    <p>
                        "How the reputation system works: trust scores, tiers, reviews, and how to build your reputation. Learn to assess counterparty risk before trading."
                    </p>
                    <span class="tutorial-tag">"Intermediate"</span>
                </div>

                <div class="tutorial-card">
                    <div class="tutorial-thumbnail">
                        <div class="play-button">"▶"</div>
                        <span class="tutorial-duration">"10:00"</span>
                    </div>
                    <h3>"How Atomic Swaps Work"</h3>
                    <p>
                        "A technical deep dive into the cryptography behind OpenDex: adaptor signatures, DLEQ proofs, blockchain confirmations, and the deterministic recovery engine."
                    </p>
                    <span class="tutorial-tag">"Advanced"</span>
                </div>

                <div class="tutorial-card">
                    <div class="tutorial-thumbnail">
                        <div class="play-button">"▶"</div>
                        <span class="tutorial-duration">"4:50"</span>
                    </div>
                    <h3>"Running a Node"</h3>
                    <p>
                        "Best practices for running an OpenDex node: network configuration, peer management, bootstrap nodes, and maintaining a healthy connection to the P2P network."
                    </p>
                    <span class="tutorial-tag">"Intermediate"</span>
                </div>

            </div>

            <div class="tutorials-note">
                <p>
                    "📹 Video tutorials are coming soon. In the meantime, check our "
                    <a href="#download">"installation guides"</a>
                    " and "
                    <a href="#faq">"FAQ"</a>
                    " for written instructions."
                </p>
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
                <a href="#faq">"FAQ"</a>
                " · "
                <a href="#blog">"Blog"</a>
                " · "
                <a href="#security">"Security"</a>
                " · "
                <a href="#bug-bounty">"Bug Bounty"</a>
                " · "
                <a href="#tutorials">"Tutorials"</a>
                " · "
                <a href="#contact">"Contact"</a>
                " · "
                <a href="#donate">"Donate"</a>
                " · "
                <a href="/legal">"Legal"</a>
            </p>
        </footer>
    }
}

/// JavaScript for copy-to-clipboard functionality
#[allow(non_snake_case)]
fn DonationScripts() -> impl IntoView {
    view! {
        <script>
            r#"
            function copyToClipboard(text, button) {
                navigator.clipboard.writeText(text).then(function() {
                    var originalText = button.textContent;
                    button.textContent = '✅ Copied!';
                    button.style.background = 'var(--accent-green)';
                    button.style.color = '#000';
                    setTimeout(function() {
                        button.textContent = originalText;
                        button.style.background = '';
                        button.style.color = '';
                    }, 2000);
                }).catch(function(err) {
                    console.error('Failed to copy:', err);
                    button.textContent = '❌ Failed';
                    setTimeout(function() {
                        button.textContent = '📋 Copy';
                    }, 2000);
                });
            }
            "#
        </script>
    }
}
