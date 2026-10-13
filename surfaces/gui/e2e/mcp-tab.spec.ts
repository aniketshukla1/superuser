// The Connectors page's MCP tab.
//
// Regression: `GET /v1/mcp` answers with the remote MCP connector list
// (`{id, name, url, connected}`), which carries none of the registry fields the
// McpServer contract declares. The row read `status.replace(...)` straight off it,
// which threw DURING RENDER — so one absent field took the entire tab down to the
// "This screen stopped working" boundary (owner report 2026-08-19). The wire rows are
// normalised at the API boundary now, so a thin row degrades instead of crashing.
import { expect } from "@playwright/test";
import { test } from "./fixtures";

const THIN_ROWS = {
  servers: [
    { connected: false, id: "notion-mcp", name: "Notion", url: "https://mcp.notion.com/mcp" },
    { connected: true, id: "linear-mcp", name: "Linear", url: "https://mcp.linear.app/mcp" },
  ],
};

async function openMcpTab(page: import("@playwright/test").Page) {
  await page.goto("/");
  // Connectors is a secondary rail destination: it lives in the More menu. Its section
  // buttons carry a count once the lists load ("MCP servers 2"), so match the name's start.
  await page.getByTestId("nav-more").click();
  await page.getByTestId("more-menu").getByTestId("nav-connectors").click();
  await page
    .getByRole("navigation", { name: "Connectors sections" })
    .getByRole("button", { name: /^MCP servers/ })
    .click();
}

test("a server row missing every registry field renders instead of taking the tab down", async ({
  page,
}) => {
  const crashes: string[] = [];
  page.on("pageerror", (e) => crashes.push(e.message));
  await page.route("**/v1/mcp", (route) =>
    route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(THIN_ROWS) }),
  );

  await openMcpTab(page);

  await expect(page.getByText("This screen stopped working")).toHaveCount(0);
  await expect(page.getByText("Notion", { exact: true })).toBeVisible();
  await expect(page.getByText("Linear", { exact: true })).toBeVisible();
  expect(crashes, `render threw: ${crashes[0]}`).toEqual([]);
});

test("a thin row states what is actually known, without inventing a sign-in it cannot do", async ({
  page,
}) => {
  await page.route("**/v1/mcp", (route) =>
    route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(THIN_ROWS) }),
  );

  await openMcpTab(page);

  // `connected` is the only status the thin shape carries, so it becomes the status —
  // and the transport reads from the url rather than being blank.
  await expect(page.getByText(/http · connected/)).toBeVisible();
  await expect(page.getByText(/http · configured/)).toBeVisible();
  // No `auth` field means no Sign in button: the tab must not offer a flow whose
  // endpoint may not exist.
  await expect(page.getByTestId("mcp-signin-Notion")).toHaveCount(0);
});
