import { HouseArt } from "./HouseArt";
import { Heart } from "./Logo";
import styles from "./SavePreview.module.scss";

/** A listing page with the extension's button, mid-save. */
export function SavePreview() {
  return (
    <figure className={styles.browser} aria-label="The Save in Home Safe button on a listing, downloading its photos">
      <div className={styles.bar} aria-hidden>
        <span className={styles.dots}>
          <span />
          <span />
          <span />
        </span>
        <span className={styles.url}>zillow.com/homedetails/14-Maple-Ln…</span>
      </div>
      <div className={styles.page} aria-hidden>
        <div className={styles.hero}>
          <HouseArt kind="gable" sky={["#bfe3f2", "#f3e7d3"]} wall="#f6efe4" roof="#c4553f" door="#2f6f6a" />
        </div>
        <div className={styles.lines}>
          <strong>$485,000</strong>
          <span className={styles.line} style={{ width: "62%" }} />
          <span className={styles.line} style={{ width: "48%" }} />
          <span className={styles.line} style={{ width: "70%" }} />
        </div>
        <div className={styles.button}>
          <span className={styles.spinner} />
          Downloading photos 12 of 24
        </div>
      </div>
      <figcaption className={styles.caption}>
        <Heart size={12} /> One click saves the listing, then its photos follow.
      </figcaption>
    </figure>
  );
}
