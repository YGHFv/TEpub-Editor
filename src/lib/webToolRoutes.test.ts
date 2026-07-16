import { describe, expect, it } from "vitest";
import { isSharedWebTool, isWebRouteToolId, webToolRoute } from "./webToolRoutes";

describe("web tool routing", () => {
  it("maps primary editor routes", () => {
    expect(webToolRoute("txt-edit")).toBe("/toolbox/text-editor");
    expect(webToolRoute("epub-read")).toBe("/toolbox/epub-editor?mode=reader");
    expect(webToolRoute("txt-epub")).toBe("/toolbox/make-epub");
  });

  it("maps parameterized processing routes", () => {
    expect(webToolRoute("font-decrypt")).toBe("/toolbox/font-process?tool=font-decrypt");
    expect(webToolRoute("image-watermark")).toBe("/toolbox/epub-advanced?tool=image-watermark");
    expect(webToolRoute("file-encrypt")).toBe("/toolbox/epub-process?tool=file-encrypt");
  });

  it("reports shared and unsupported tools", () => {
    expect(isSharedWebTool("epub-merge")).toBe(true);
    expect(isWebRouteToolId("epub-diagnose")).toBe(true);
    expect(webToolRoute("unknown")).toBe("#");
  });
});
