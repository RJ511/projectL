import { describe, expect, it } from "vitest";
import {
  buildFallbackInlineConcept,
  extractInlineConceptGraph,
  parseInlineConceptToken,
} from "./inlineConcepts";

describe("inlineConcepts", () => {
  it("parses inline concept tokens with prereqs and weight", () => {
    expect(
      parseInlineConceptToken("Álgebra linear:bases, matrizes [0.7]"),
    ).toEqual({
      conceptLabel: "Álgebra linear",
      prereqLabels: ["bases", "matrizes"],
      coverageWeight: 0.7,
    });
  });

  it("extracts weighted declared concepts from content", () => {
    const graph = extractInlineConceptGraph(
      "# Aula\n;;;Autovalores [0.8];;;\n;;;Diagonalização:Autovalores;;;",
      "Math",
    );

    expect(graph.conceptIds).toContain("math.inline.autovalores");
    expect(graph.conceptIds).toContain("math.inline.diagonalizacao");
    expect(graph.edges).toContainEqual({
      prereq_id: "math.inline.autovalores",
      target_id: "math.inline.diagonalizacao",
    });
    expect(
      graph.concepts.find((concept) => concept.id === "math.inline.autovalores")
        ?.coverageWeight,
    ).toBe(0.8);
  });

  it("builds a fallback file concept when no inline markup exists", () => {
    expect(buildFallbackInlineConcept("Álgebra Linear.md", "Math")).toEqual({
      id: "math.file.algebra-linear-md",
      name: "Álgebra Linear.md",
      description: "Conceito de ficheiro para Álgebra Linear.md",
      coverageWeight: 1.0,
    });
  });
});
