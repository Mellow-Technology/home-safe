import { Logo } from "./Logo";
import { HouseArt, type HouseArtProps } from "./HouseArt";
import styles from "./AppPreview.module.scss";

interface Listing {
  price: string;
  perSqft: string;
  specs: string;
  address: string;
  status: string;
  art: HouseArtProps;
}

// Made-up homes, not real listings.
const LISTINGS: Listing[] = [
  {
    price: "$485,000",
    perSqft: "$296/sq ft",
    specs: "3 bd · 2 ba · 1,640 sq ft",
    address: "14 Maple Lane, Portland, OR",
    status: "For sale",
    art: { kind: "gable", sky: ["#bfe3f2", "#f3e7d3"], wall: "#f6efe4", roof: "#c4553f", door: "#2f6f6a" },
  },
  {
    price: "$372,500",
    perSqft: "$345/sq ft",
    specs: "2 bd · 1 ba · 1,080 sq ft",
    address: "52 Willow Court, Asheville, NC",
    status: "For sale",
    art: { kind: "cottage", sky: ["#f7c9b5", "#fbe7d0"], wall: "#fbf3e6", roof: "#6d7f8f", door: "#c73a52" },
  },
  {
    price: "$629,900",
    perSqft: "$285/sq ft",
    specs: "4 bd · 3 ba · 2,210 sq ft",
    address: "208 Harbor Street, Tacoma, WA",
    status: "Pending",
    art: { kind: "modern", sky: ["#3d4a77", "#c98b8b"], wall: "#e9e1d6", roof: "#4b5563", door: "#3b2b26", evening: true },
  },
  {
    price: "$545,000",
    perSqft: "$288/sq ft",
    specs: "3 bd · 2.5 ba · 1,890 sq ft",
    address: "9 Orchard Road, Madison, WI",
    status: "Coming soon",
    art: { kind: "tall", sky: ["#cfe8f0", "#eef4e6"], wall: "#9fc3cf", roof: "#3f4a5a", door: "#f6efe4" },
  },
  {
    price: "$410,000",
    perSqft: "$289/sq ft",
    specs: "3 bd · 2 ba · 1,420 sq ft",
    address: "301 Linden Avenue, Durham, NC",
    status: "For sale",
    art: { kind: "cottage", sky: ["#c6e2d6", "#f2ecd9"], wall: "#f3d9b8", roof: "#8a4b3a", door: "#2f6f6a" },
  },
  {
    price: "$699,000",
    perSqft: "$273/sq ft",
    specs: "4 bd · 3.5 ba · 2,560 sq ft",
    address: "77 Cedar Hollow, Boulder, CO",
    status: "For sale",
    art: { kind: "gable", sky: ["#f5b88f", "#f9e2c4"], wall: "#e8dccb", roof: "#5b6b5a", door: "#a92f45", evening: true },
  },
];

/** The app's card view, drawn in HTML: a picture, not a screenshot. */
export function AppPreview() {
  return (
    <figure className={styles.window} aria-label="The Home Safe app, showing saved homes as cards">
      <div className={styles.titlebar} aria-hidden>
        <span />
        <span />
        <span />
      </div>
      <div className={styles.toolbar} aria-hidden>
        <div className={styles.brand}>
          <Logo size={18} />
          Home Safe
        </div>
        <div className={styles.search}>Search homes</div>
        <div className={styles.switcher}>
          <span className={styles.on}>
            <GridIcon />
          </span>
          <span>
            <TableIcon />
          </span>
        </div>
      </div>
      <ul className={styles.cards}>
        {LISTINGS.map((home) => (
          <li key={home.address} className={styles.card}>
            <div className={styles.photo}>
              <HouseArt {...home.art} />
            </div>
            <div className={styles.info}>
              <div className={styles.priceRow}>
                <strong>{home.price}</strong>
                <span>{home.perSqft}</span>
              </div>
              <div className={styles.specs}>
                {home.specs}
                <span className={styles.status}>{home.status}</span>
              </div>
              <div className={styles.address}>{home.address}</div>
            </div>
          </li>
        ))}
      </ul>
      <div className={styles.statusbar} aria-hidden>
        <span>6 homes</span>
        <span>API · 127.0.0.1:47321</span>
      </div>
    </figure>
  );
}

function GridIcon() {
  return (
    <svg width="12" height="12" viewBox="0 0 12 12" fill="currentColor">
      <rect width="5" height="5" rx="1" />
      <rect x="7" width="5" height="5" rx="1" />
      <rect y="7" width="5" height="5" rx="1" />
      <rect x="7" y="7" width="5" height="5" rx="1" />
    </svg>
  );
}

function TableIcon() {
  return (
    <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" strokeWidth="1.3">
      <rect x=".65" y=".65" width="10.7" height="10.7" rx="1.5" />
      <path d="M.65 4.3h10.7M.65 7.9h10.7M4.3 4.3v7" />
    </svg>
  );
}
