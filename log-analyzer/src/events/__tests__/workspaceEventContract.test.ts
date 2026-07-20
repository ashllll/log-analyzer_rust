/**
 * workspace-event 跨语言契约测试
 *
 * 夹具 src/events/__fixtures__/workspace-event-contract.json 是线上 payload
 * 形状的唯一事实源：后端 state_sync/contract_tests.rs 锁定 serde 序列化输出
 * 与该夹具一致，本测试锁定前端 zod schema 接受同一夹具。
 * 任一侧单方面漂移都会使对应测试失败。
 */
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { WorkspaceEventSchema } from "../types";

const fixture: unknown[] = JSON.parse(
  readFileSync(
    join(__dirname, "../__fixtures__/workspace-event-contract.json"),
    "utf8"
  )
);

describe("workspace-event wire contract", () => {
  it("fixture covers every status variant", () => {
    const statuses = fixture.map(
      (entry) => (entry as { status: { status: string } }).status.status
    );
    expect(statuses).toEqual([
      "Idle",
      "Processing",
      "Completed",
      "Failed",
      "Cancelled",
    ]);
  });

  it.each(fixture.map((_, index) => [index]))(
    "schema accepts backend payload at fixture index %i",
    (index) => {
      const result = WorkspaceEventSchema.safeParse(fixture[index]);
      expect(result.success).toBe(true);
    }
  );

  it("schema rejects the historical broken shape (status as plain string)", () => {
    // 回归固定：旧 schema 期望字符串 status，而后端实际发送嵌套对象，
    // 导致所有 workspace-event 被静默丢弃。该形状必须继续被拒绝。
    const legacyBrokenShape = {
      type: "StatusChanged",
      workspace_id: "ws-1",
      status: "Completed",
    };
    expect(WorkspaceEventSchema.safeParse(legacyBrokenShape).success).toBe(
      false
    );
  });

  it("schema rejects unknown workspace event types", () => {
    const unknownType = {
      type: "Created",
      workspace_id: "ws-1",
    };
    expect(WorkspaceEventSchema.safeParse(unknownType).success).toBe(false);
  });
});
