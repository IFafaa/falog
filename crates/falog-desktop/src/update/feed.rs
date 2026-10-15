//! What the latest release is and which of its files this computer needs. The release list comes from
//! GitHub's Atom feed of releases rather than its REST API, which allows each address only 60 requests
//! an hour without a token.

use std::cmp::Ordering;
use std::fmt;

/// The repository releases are published in (see .spec/releases.md).
pub const REPO: &str = "IFafaa/falog";

/// The feed of every release, newest first, pre-releases included.
pub fn feed_url() -> String {
    format!("https://github.com/{REPO}/releases.atom")
}

/// Where a file of the release tagged `tag` is downloaded from.
pub fn asset_url(tag: &str, name: &str) -> String {
    format!("https://github.com/{REPO}/releases/download/{tag}/{name}")
}

/// A semantic version: `0.2.0`, or `0.3.0-beta.1` for a preview.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Version {
    major: u64,
    minor: u64,
    patch: u64,
    /// The pre-release identifiers after `-` (`beta`, `1`); empty for a stable release.
    pre: Vec<String>,
}

impl Version {
    /// Parses `1.2.3`, `v1.2.3` or `1.2.3-rc.1`; build metadata (`+...`) is ignored.
    pub fn parse(text: &str) -> Option<Self> {
        let text = text.trim().strip_prefix('v').unwrap_or(text.trim());
        let text = text.split('+').next()?;
        let (core, pre) = match text.split_once('-') {
            Some((core, pre)) => (core, pre.split('.').map(str::to_owned).collect()),
            None => (text, Vec::new()),
        };
        let mut numbers = core.split('.').map(|n| n.parse::<u64>().ok());
        let version = Self {
            major: numbers.next()??,
            minor: numbers.next()??,
            patch: numbers.next()??,
            pre,
        };
        (numbers.next().is_none() && version.pre.iter().all(|p| !p.is_empty())).then_some(version)
    }

    /// The version of this build of Falog.
    pub fn current() -> Self {
        Self::parse(env!("CARGO_PKG_VERSION")).unwrap_or(Self {
            major: 0,
            minor: 0,
            patch: 0,
            pre: Vec::new(),
        })
    }

    pub fn is_preview(&self) -> bool {
        !self.pre.is_empty()
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.major, self.minor, self.patch)
            .cmp(&(other.major, other.minor, other.patch))
            .then_with(|| match (self.pre.is_empty(), other.pre.is_empty()) {
                (true, true) => Ordering::Equal,
                // A release comes after its own pre-releases.
                (true, false) => Ordering::Greater,
                (false, true) => Ordering::Less,
                (false, false) => compare_pre(&self.pre, &other.pre),
            })
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Semver precedence of pre-release identifiers: numbers numerically and below words, words as text,
/// and a shorter list first when one is a prefix of the other.
fn compare_pre(a: &[String], b: &[String]) -> Ordering {
    for (x, y) in a.iter().zip(b) {
        let order = match (x.parse::<u64>(), y.parse::<u64>()) {
            (Ok(x), Ok(y)) => x.cmp(&y),
            (Ok(_), Err(_)) => Ordering::Less,
            (Err(_), Ok(_)) => Ordering::Greater,
            (Err(_), Err(_)) => x.cmp(y),
        };
        if order != Ordering::Equal {
            return order;
        }
    }
    a.len().cmp(&b.len())
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        if !self.pre.is_empty() {
            write!(f, "-{}", self.pre.join("."))?;
        }
        Ok(())
    }
}

/// A published release newer than this build.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Release {
    pub tag: String,
    pub version: Version,
}

/// The release tags in the feed, in its order: each entry links to `.../releases/tag/<tag>`.
pub fn tags_in_feed(feed: &str) -> Vec<String> {
    let mut tags: Vec<String> = Vec::new();
    for part in feed.split("/releases/tag/").skip(1) {
        let tag: String = part
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '+' | '_'))
            .collect();
        if !tag.is_empty() && !tags.contains(&tag) {
            tags.push(tag);
        }
    }
    tags
}

/// The newest release in `tags` that is newer than `current`; previews only when `preview` is on.
pub fn newest(tags: &[String], current: &Version, preview: bool) -> Option<Release> {
    tags.iter()
        .filter_map(|tag| Version::parse(tag).map(|version| (tag, version)))
        .filter(|(_, version)| preview || !version.is_preview())
        .filter(|(_, version)| version > current)
        .max_by(|(_, a), (_, b)| a.cmp(b))
        .map(|(tag, version)| Release {
            tag: tag.clone(),
            version,
        })
}

/// The download this computer installs, as named in every release (.spec/releases.md).
pub fn asset_name() -> Option<&'static str> {
    if cfg!(all(windows, target_arch = "x86_64")) {
        Some("Falog-Setup-x64.exe")
    } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        Some("Falog-macOS-arm64.dmg")
    } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
        Some("Falog-macOS-x64.dmg")
    } else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        Some("falog-linux-x86_64.tar.gz")
    } else {
        None
    }
}

/// The SHA-256 of `name` in a release's `SHA256SUMS` (`<hex>  <name>` per line).
pub fn checksum_for(sums: &str, name: &str) -> Option<String> {
    sums.lines().find_map(|line| {
        let (hash, file) = line.split_once(char::is_whitespace)?;
        let file = file.trim_start().trim_start_matches('*');
        (file == name && hash.len() == 64 && hash.chars().all(|c| c.is_ascii_hexdigit()))
            .then(|| hash.to_ascii_lowercase())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(text: &str) -> Version {
        Version::parse(text).unwrap()
    }

    #[test]
    fn orders_versions_like_semver() {
        assert!(v("0.1.1") > v("0.1.0"));
        assert!(v("0.10.0") > v("0.9.9"));
        assert!(v("1.0.0") > v("1.0.0-rc.1"));
        assert!(v("1.0.0-rc.1") > v("1.0.0-beta.11"));
        assert!(v("1.0.0-beta.11") > v("1.0.0-beta.2"));
        assert!(v("1.0.0-beta.2") > v("1.0.0-beta"));
        assert_eq!(v("v0.2.0"), v("0.2.0+build.5"));
        for bad in ["", "1.2", "1.2.3.4", "a.b.c", "1.2.3-", "1.2.3-beta..1"] {
            assert_eq!(Version::parse(bad), None, "{bad}");
        }
        assert_eq!(v("0.3.0-beta.1").to_string(), "0.3.0-beta.1");
    }

    #[test]
    fn orders_previews_by_semver_precedence() {
        // The example from semver.org, oldest first.
        let chain = [
            "1.0.0-alpha",
            "1.0.0-alpha.1",
            "1.0.0-alpha.beta",
            "1.0.0-beta",
            "1.0.0-beta.2",
            "1.0.0-beta.11",
            "1.0.0-rc.1",
            "1.0.0",
        ];
        for pair in chain.windows(2) {
            assert!(v(pair[0]) < v(pair[1]), "{} < {}", pair[0], pair[1]);
        }
        assert!(v("1.0.0-rc.1").is_preview());
        assert!(!v("1.0.0").is_preview());
    }

    #[test]
    fn knows_the_version_of_this_build() {
        // Unreadable, it would count as 0.0.0 and every release would look newer.
        assert_eq!(Version::current().to_string(), env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn a_preview_build_is_offered_the_stable_release_it_led_to() {
        let tags: Vec<String> = ["v0.3.0", "v0.3.0-rc.2"].map(String::from).to_vec();
        let running = v("0.3.0-rc.1");
        assert_eq!(newest(&tags, &running, false).unwrap().tag, "v0.3.0");
        assert_eq!(
            newest(&tags, &running, true).unwrap().tag,
            "v0.3.0",
            "the release beats its previews"
        );
        // The feed's order does not matter.
        let shuffled: Vec<String> = ["v0.2.0", "v0.4.1", "v0.4.0"].map(String::from).to_vec();
        assert_eq!(newest(&shuffled, &v("0.3.0"), false).unwrap().tag, "v0.4.1");
    }

    #[test]
    fn reads_checksums_written_either_way() {
        let hash = "AB".repeat(32);
        // `sha256sum --binary` marks the file with `*`; some tools write upper case or a tab.
        let sums = format!(
            "{hash} *Falog-Setup-x64.exe\n{}\tfalog-linux-x86_64.tar.gz\n",
            "c".repeat(64)
        );
        assert_eq!(checksum_for(&sums, "Falog-Setup-x64.exe"), Some("ab".repeat(32)));
        assert_eq!(
            checksum_for(&sums, "falog-linux-x86_64.tar.gz"),
            Some("c".repeat(64))
        );
        assert_eq!(checksum_for(&sums, "Falog-Setup"), None, "whole names only");
        let not_hex = format!("{}  Falog-Setup-x64.exe", "z".repeat(64));
        assert_eq!(checksum_for(&not_hex, "Falog-Setup-x64.exe"), None);
    }

    #[test]
    fn reads_the_tags_of_the_feed() {
        let feed = r#"<feed><entry><id>tag:github.com,2008:Repository/1/v0.2.0-beta.1</id>
            <link rel="alternate" type="text/html" href="https://github.com/IFafaa/falog/releases/tag/v0.2.0-beta.1"/>
            </entry><entry><link rel="alternate" href="https://github.com/IFafaa/falog/releases/tag/v0.1.1"/>
            </entry></feed>"#;
        assert_eq!(tags_in_feed(feed), ["v0.2.0-beta.1", "v0.1.1"]);
    }

    #[test]
    fn offers_the_newest_release_of_the_channel() {
        let tags: Vec<String> = ["v0.2.0-beta.1", "v0.1.1", "v0.1.0", "nightly"]
            .map(String::from)
            .to_vec();
        let stable = newest(&tags, &v("0.1.0"), false).unwrap();
        assert_eq!((stable.tag.as_str(), stable.version), ("v0.1.1", v("0.1.1")));
        assert_eq!(newest(&tags, &v("0.1.0"), true).unwrap().tag, "v0.2.0-beta.1");
        assert_eq!(newest(&tags, &v("0.1.1"), false), None, "up to date");
        assert_eq!(newest(&tags, &v("0.3.0"), true), None, "newer than any release");
    }

    #[test]
    fn finds_a_file_in_the_checksums() {
        let hash = "a".repeat(64);
        let sums = format!(
            "{}  falog-linux-x86_64.tar.gz\n{hash}  Falog-Setup-x64.exe\n",
            "b".repeat(64)
        );
        assert_eq!(checksum_for(&sums, "Falog-Setup-x64.exe"), Some(hash));
        assert_eq!(checksum_for(&sums, "Falog-macOS-arm64.dmg"), None);
        assert_eq!(
            checksum_for("short  Falog-Setup-x64.exe", "Falog-Setup-x64.exe"),
            None
        );
    }
}
