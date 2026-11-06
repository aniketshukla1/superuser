import { test, expect } from "./fixtures";

// ChatGPT-style mentions: `@ops …` hands that one message to another assistant, which
// answers it in this conversation. The "@" menu offers the assistants, and the reply
// says who gave it (the fake agent mirrors `coworker::mentioned`).
test("an @ mention offers the other assistants and labels the reply with who answered", async ({ page }) => {
  await page.goto("/");
  const box = page.getByTestId("composer-input");
  await box.fill("@op");
  const row = page.getByRole("option", { name: /^@ops/ });
  await expect(row).toBeVisible();
  await expect(row).toHaveAttribute("title", /Ops Assistant — Incidents and runbooks/);
  await row.click();
  await expect(box).toHaveValue("@ops ");

  await box.fill("@ops why did last night's backup fail?");
  await page.getByRole("button", { name: "Send" }).click();
  await expect(page.getByText(/Echo: @ops why did last night's backup fail\?/)).toBeVisible();
  await expect(page.getByTestId("reply-from")).toHaveText("Ops");

  // The next message is the conversation's own again: no label on its reply.
  await box.fill("thanks");
  await page.getByRole("button", { name: "Send" }).click();
  await expect(page.getByText(/Echo: thanks/)).toBeVisible();
  await expect(page.getByTestId("reply-from")).toHaveCount(1);
});
