// Where things live, in one place, so a rename changes one file.
export const REPO_URL = "https://github.com/aniketshukla1/superuser";
export const repoFile = (path: string) => `${REPO_URL}/blob/main/${path}`;

// The contact form posts to a Formspree endpoint (https://formspree.io/f/<id>), set at build
// time from the CONTACT_FORM_ENDPOINT repository variable (see .github/workflows/site.yml).
// The address the messages reach lives in that Formspree account. It is never written into
// this repository or the page, which is the point: a mailto: link would hand it to every
// scraper that loads the site.
export const CONTACT_ENDPOINT: string = import.meta.env.VITE_CONTACT_ENDPOINT ?? "";
