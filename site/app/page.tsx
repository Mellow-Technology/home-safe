import { Button } from "gui-mellow/Button";
import { Card } from "gui-mellow/Card";
import { Credit } from "gui-mellow/Credit";
import { CtaBanner } from "gui-mellow/CtaBanner";
import { Feature, FeatureGrid } from "gui-mellow/FeatureGrid";
import { Hero } from "gui-mellow/Hero";
import { Section } from "gui-mellow/Section";
import { SiteFooter } from "gui-mellow/SiteFooter";
import { SiteHeader } from "gui-mellow/SiteHeader";
import { Steps } from "gui-mellow/Steps";
import { Terminal } from "gui-mellow/Terminal";
import { ThemeToggle } from "gui-mellow/ThemeToggle";
import { AppPreview } from "@/components/AppPreview";
import { Heart, Logo } from "@/components/Logo";
import { SavePreview } from "@/components/SavePreview";
import styles from "./page.module.scss";

const REPO = "https://github.com/Mellow-Technology/home-safe";
const RELEASES = `${REPO}/releases`;
// The release files carry no version, so these always fetch the newest.
const DOWNLOAD_APP = `${RELEASES}/latest/download/Home-Safe-macos.dmg`;
const DOWNLOAD_EXTENSION = `${RELEASES}/latest/download/Home-Safe-extension.zip`;

/** An eyebrow in the body face. */
function Kicker({ children }: { children: React.ReactNode }) {
  return <span className={styles.kicker}>{children}</span>;
}

function Brand() {
  return (
    <a href="#top" className={styles.brand}>
      <Logo />
      Home Safe
    </a>
  );
}

export default function Home() {
  return (
    <>
      <SiteHeader
        brand={<Brand />}
        items={[
          { href: "#how", label: "How it works" },
          { href: "#features", label: "Features" },
          { href: "#private", label: "Privacy" },
          { href: "#get-started", label: "Get started" },
        ]}
        actions={
          <>
            {/* On phones the theme follows the system's, and the banner and footer link to GitHub. */}
            <span className={styles.wideOnly}>
              <ThemeToggle />
            </span>
            <span className={styles.wideOnly}>
              <Button href={REPO} variant="secondary" size="sm" icon={<GitHubIcon />}>
                GitHub
              </Button>
            </span>
          </>
        }
      />

      <main id="top">
        <Hero
          size="full"
          align="center"
          eyebrow={<Kicker>Free and open source · Private by design</Kicker>}
          title={
            <>
              Home is where <em>the heart</em> is
            </>
          }
          subtitle="Keep every home you fall for while you search. Home Safe saves a listing, with every photo, in one click, to a cozy little inventory on your own computer."
          actions={
            <>
              <Button href="#get-started" size="lg" icon={<Heart size={16} />}>
                Get Home Safe
              </Button>
              <Button href="#how" variant="secondary" size="lg">
                See how it works
              </Button>
            </>
          }
        >
          <AppPreview />
        </Hero>

        <Section
          id="how"
          size="wide"
          eyebrow={<Kicker>How it works</Kicker>}
          title="From “ooh” to saved in one click"
          description="No copying links into a spreadsheet, no screenshots, no losing the one you loved when the listing comes down."
        >
          <div className={styles.split}>
            <Steps
              steps={[
                {
                  title: "Add the extension",
                  body: "It puts a Save in Home Safe button on Zillow listings, in Chrome or Firefox.",
                },
                {
                  title: "Save the homes you love",
                  body: "One click sends the listing to the app: price, beds and baths, size, HOA, Zestimate and the description. Its photos download in the background, and the button shows how far along they are.",
                },
                {
                  title: "Come back to them any time",
                  body: "Browse your homes as cards or a sortable table, open one for its photos and facts, and compare at your own pace. Save a home again to bring its details up to date.",
                },
              ]}
            />
            <SavePreview />
          </div>
        </Section>

        <Section
          id="features"
          size="wide"
          eyebrow={<Kicker>Features</Kicker>}
          title="Everything about the homes you love, in one place"
          description="A calm, quick desktop app for the most exciting search of your life."
        >
          <FeatureGrid min="300px" className={styles.features}>
            <Feature index={<Heart />} title="Every photo, kept">
              Photos are downloaded to your computer when you save, so they stay with you after the listing changes or
              disappears. Open any of them full size and step through with the arrow keys.
            </Feature>
            <Feature index={<Heart />} title="Cards or a table">
              Flip through photo cards, or switch to a table and sort by price, price per square foot, beds, baths or
              size. Search narrows either view as you type.
            </Feature>
            <Feature index={<Heart />} title="A page for every home">
              Each home gets its own page: a photo mosaic, the price and price per square foot, year built, lot size,
              HOA, Zestimate and rent estimate, and the full description.
            </Feature>
            <Feature index={<Heart />} title="Always ready to save">
              On a Mac, Home Safe lives in the menu bar. Close the window and it keeps listening for the extension, and
              it can start when you log in.
            </Feature>
            <Feature index={<Heart />} title="Nothing lost">
              Every save also keeps a snapshot of the listing as it was, with its price and status, so a home&apos;s
              history builds up quietly while you decide.
            </Feature>
            <Feature index={<Heart />} title="Easy to let go">
              Changed your mind? Remove a home from the listing page or the app, and its photos and history go with
              it.
            </Feature>
          </FeatureGrid>
        </Section>

        <Section
          id="private"
          size="wide"
          eyebrow={<Kicker>Privacy</Kicker>}
          title="Your home search stays at home"
          description="Where you might live is personal. Home Safe keeps it that way: no account, no cloud, no tracking."
        >
          <ol className={styles.flow} aria-label="Where your data goes">
            <li>
              <strong>Your browser</strong>
              <span>The extension reads the listing you&apos;re looking at</span>
            </li>
            <li>
              <strong>Your computer</strong>
              <span>
                It hands it to the app at <code>127.0.0.1</code>, which only answers the extension
              </span>
            </li>
            <li>
              <strong>Your disk</strong>
              <span>Homes in a local database, photos in a folder you can open</span>
            </li>
          </ol>
          <p className={styles.note}>
            Nothing is sent anywhere else. The only requests Home Safe makes are to download the photos of homes you
            save.
          </p>
        </Section>

        <Section
          id="get-started"
          size="wide"
          eyebrow={<Kicker>Get started</Kicker>}
          title="Make yourself at home"
          description="Home Safe is free for Macs with macOS 12 or later, Apple silicon or Intel. Setting it up takes a couple of minutes."
        >
          <div className={styles.split}>
            <Steps
              steps={[
                {
                  title: "Download and install",
                  body: (
                    <>
                      Open <code>Home-Safe-macos.dmg</code> and drag Home Safe into your Applications folder.
                    </>
                  ),
                },
                {
                  title: "Open it the first time",
                  body: (
                    <>
                      Home Safe isn&apos;t notarized by Apple yet, so the first time you open it macOS says it can&apos;t
                      check it for malware. Click <strong>Done</strong>, then go to{" "}
                      <strong>System Settings → Privacy &amp; Security</strong>, scroll down and click{" "}
                      <strong>Open Anyway</strong> beside Home Safe. You only need to do this once; after that it opens
                      like any other app, and its house appears in your menu bar.
                    </>
                  ),
                },
                {
                  title: "Add the extension",
                  body: (
                    <>
                      Unzip <code>Home-Safe-extension.zip</code>. In Chrome, open <code>chrome://extensions</code>, turn
                      on Developer mode, click <strong>Load unpacked</strong> and choose the folder. In Firefox, open{" "}
                      <code>about:debugging</code>, choose <strong>This Firefox</strong> and load its{" "}
                      <code>manifest.json</code>. Store versions are on the way.
                    </>
                  ),
                },
              ]}
            />
            <div className={styles.download}>
              <Card
                icon={<Logo size={40} />}
                title="Home Safe for Mac"
                footer={
                  <>
                    <a href={RELEASES}>Release notes and older versions</a>
                  </>
                }
              >
                <p>The app, ready to drag into Applications, and the extension for Chrome and Firefox.</p>
                <div className={styles.downloadActions}>
                  <Button href={DOWNLOAD_APP} icon={<Heart size={16} />}>
                    Download for Mac
                  </Button>
                  <Button href={DOWNLOAD_EXTENSION} variant="secondary">
                    Get the extension
                  </Button>
                </div>
              </Card>
              <p className={styles.fromSource}>
                Rather build it yourself? With <a href="https://rustup.rs">Rust</a> installed:
              </p>
              <Terminal
                label="Terminal"
                lines={[
                  "git clone https://github.com/Mellow-Technology/home-safe",
                  "cd home-safe",
                  "cargo run --release -p home-safe",
                ]}
              />
            </div>
          </div>
        </Section>

        <section className={styles.cta}>
          <CtaBanner
            title="Find the one, and keep the ones you loved along the way"
            actions={
              <>
                <Button href={DOWNLOAD_APP} icon={<Heart size={16} />}>
                  Download for Mac
                </Button>
                <Button href={REPO} variant="secondary" icon={<GitHubIcon />}>
                  View on GitHub
                </Button>
              </>
            }
          >
            Free and open source under the LGPL-3.0. Your homes stay on your computer.
          </CtaBanner>
        </section>
      </main>

      <SiteFooter
        brand={<Brand />}
        columns={[
          {
            heading: "Project",
            links: [
              { href: REPO, label: "Source code" },
              { href: RELEASES, label: "Releases" },
              { href: `${REPO}/issues`, label: "Issues" },
              { href: `${REPO}/blob/main/COPYING.LESSER`, label: "License" },
            ],
          },
          {
            heading: "Get started",
            links: [
              { href: DOWNLOAD_APP, label: "Download for Mac" },
              { href: "#get-started", label: "Install" },
              { href: "#how", label: "How it works" },
              { href: "#private", label: "Privacy" },
            ],
          },
        ]}
      >
        <p>
          Made with <span className={styles.love}>♥</span> for house hunters. © 2026 Mellow Technology.
        </p>
        <p>Home Safe isn&apos;t affiliated with or endorsed by Zillow.</p>
        <p>
          <Credit />
        </p>
      </SiteFooter>
    </>
  );
}

function GitHubIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" aria-hidden focusable="false" fill="currentColor">
      <path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0016 8c0-4.42-3.58-8-8-8z" />
    </svg>
  );
}
