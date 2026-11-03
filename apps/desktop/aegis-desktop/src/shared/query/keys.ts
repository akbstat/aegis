import type { StructureLevel } from "../api/types";

// Query key factory. Keys are tuples typed `as const` so downstream
// `useQuery({ queryKey: queryKeys.x.y() })` gets exact tuple inference.
// All hooks and invalidations reference keys through this module —
// never inline arrays — so a typo breaks one site at a time.

export const queryKeys = {
  auth: {
    loginStatus: () => ["auth", "loginStatus"] as const,
  },
  bootstrap: {
    health: () => ["bootstrap", "health"] as const,
  },
  user: {
    current: () => ["user", "current"] as const,
    domainIdentity: () => ["user", "domainIdentity"] as const,
    list: () => ["user", "list"] as const,
  },
  project: {
    all: () => ["project", "list"] as const,
    byCode: (code: string) => ["project", "byCode", code] as const,
  },
  product: {
    all: () => ["product", "list"] as const,
  },
  terminology: {
    versions: () => ["terminology", "versions"] as const,
    version: (id: number) => ["terminology", "version", id] as const,
    codeLists: (versionId: number, fragment: string) =>
      ["terminology", "codeLists", versionId, fragment] as const,
    codeList: (id: number) => ["terminology", "codeList", id] as const,
    codeListBySubmissionValue: (versionId: number, submissionValue: string) =>
      ["terminology", "codeListBySubmissionValue", versionId, submissionValue] as const,
    codeItems: (codelistId: number, fragment: string) =>
      ["terminology", "codeItems", codelistId, fragment] as const,
    codeItemsByCode: (versionId: number, code: string) =>
      ["terminology", "codeItemsByCode", versionId, code] as const,
    codeItemsGlobal: (versionId: number, fragment: string) =>
      ["terminology", "codeItemsGlobal", versionId, fragment] as const,
  },
  domainModel: {
    sdtmVersions: () => ["domainModel", "sdtmVersions"] as const,
    sdtmDomains: (versionId: number) =>
      ["domainModel", "sdtmDomains", versionId] as const,
    sdtmDomain: (id: number) =>
      ["domainModel", "sdtmDomain", id] as const,
    sdtmVariables: (domainId: number) =>
      ["domainModel", "sdtmVariables", domainId] as const,
  },
  crf: {
    versionsByProject: (projectCode: string) =>
      ["crf", "versionsByProject", projectCode] as const,
    formsByVersion: (versionId: number) =>
      ["crf", "formsByVersion", versionId] as const,
    form: (id: number) =>
      ["crf", "form", id] as const,
    formDetail: (id: number) =>
      ["crf", "formDetail", id] as const,
    item: (id: number) =>
      ["crf", "item", id] as const,
    option: (id: number) =>
      ["crf", "option", id] as const,
    unit: (id: number) =>
      ["crf", "unit", id] as const,
    searchFormsByVersion: (v: number, f: string) =>
      ["crf", "searchFormsByVersion", v, f] as const,
    searchItemsByVersion: (v: number, f: string) =>
      ["crf", "searchItemsByVersion", v, f] as const,
    searchUnitsByVersion: (v: number, f: string) =>
      ["crf", "searchUnitsByVersion", v, f] as const,
    searchOptionsByVersion: (v: number, f: string) =>
      ["crf", "searchOptionsByVersion", v, f] as const,
    searchDomainAnnotationsByVersion: (v: number, f: string) =>
      ["crf", "searchDomainAnnotationsByVersion", v, f] as const,
    searchAnnotationsByVersion: (v: number, f: string) =>
      ["crf", "searchAnnotationsByVersion", v, f] as const,
  },
  mission: {
    byProject: (projectCode: string, kind: string) =>
      ["mission", "byProject", projectCode, kind] as const,
    issuesByMission: (missionId: number) =>
      ["mission", "issuesByMission", missionId] as const,
  },
  structures: {
    list: (level: StructureLevel) => ["structures", "list", level] as const,
    detail: (id: number) => ["structures", "detail", id] as const,
  },
} as const;