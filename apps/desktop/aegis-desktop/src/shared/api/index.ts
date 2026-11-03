import { invoke } from "@tauri-apps/api/core";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";

import type {
  Annotation,
  AnnotationListResponse,
  AppendCommentInput,
  AssigneeDataArg,
  AssigneeViewResponse,
  CodeItemListQuery,
  CodeItemListResponse,
  CodeItemView,
  CodeListListQuery,
  CodeListView,
  CreateAnnotationInput,
  CreateCodeItemInput,
  CreateCodeListInput,
  CreateIssueInput,
  CreateCrfFormInput,
  CreateDomainAnnotationInput,
  CreateMissionInput,
  CreateProjectInput,
  CreateSdtmDomainInput,
  CreateSdtmVariableInput,
  CreateTerminologyVersionInput,
  CreateUserInput,
  CrfEdcType,
  CrfForm,
  CrfFormDetail,
  CrfFormListResponse,
  CrfItem,
  CrfOption,
  CrfOptionListResponse,
  CrfUnit,
  CrfUnitListResponse,
  CrfVersion,
  CrfVersionListResponse,
  DomainAnnotation,
  DomainAnnotationListResponse,
  Identity,
  IssueListQuery,
  IssueState,
  IssueViewResponse,
  MissionKind,
  MissionViewResponse,
  PagedCodeItemListResponse,
  PagedCodeListListResponse,
  ProjectView,
  RegisterUserInput,
  RegisterUserResponse,
  SdtmDomainListResponse,
  SdtmDomainView,
  SdtmVariableListResponse,
  SdtmVariableView,
  SdtmVersionListResponse,
  SdtmVersionView,
  StructureLevel,
  StructureNodeInput,
  StructureNodeView,
  StructureSummaryView,
  StructureView,
  TerminologyKind,
  TerminologyVersionView,
  UpdateAnnotationInput,
  UpdateCodeItemInput,
  UpdateCodeListInput,
  UpdateCrfFormInput,
  UpdateCrfItemInput,
  UpdateCrfOptionInput,
  UpdateCrfUnitInput,
  UpdateDomainAnnotationInput,
  UpdateProjectBody,
  UpdateSdtmDomainInput,
  UpdateSdtmVariableInput,
  UpdateTerminologyVersionInput,
  UpdateUserBody,
  UpdateUserCredentialInput,
  UserCredentialView,
  UserView,
  WebviewLogLevel,
} from "./types";

// Thin wrapper that loosens the `args` parameter type from
// `InvokeArgs` (= `Record<string, unknown>`) to `unknown` so typed input
// interfaces (CreateUserInput, etc.) flow through without per-call casts.
function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (args === undefined) {
    return invoke<T>(cmd);
  }
  return invoke<T>(cmd, args);
}

export const api = {
  // auth
  login: (code: string, password: string): Promise<void> =>
    call<void>("login", { code, password }),
  loginDomain: (): Promise<void> => call<void>("login_domain"),
  isLoggedIn: (): Promise<boolean> => call<boolean>("is_logged_in"),
  refresh: (): Promise<void> => call<void>("refresh"),
  logout: (): Promise<void> => call<void>("logout"),

  // identity
  getDomainUserInfo: (): Promise<Identity> =>
    call<Identity>("get_domain_user_info"),

  // user-credential
  registerUser: (input: RegisterUserInput): Promise<RegisterUserResponse> =>
    call<RegisterUserResponse>("register_user", { ...input }),
  updateUserCredential: (
    input: UpdateUserCredentialInput,
  ): Promise<UserCredentialView> =>
    call<UserCredentialView>("update_user_credential", { ...input }),

  // user
  createUser: (input: CreateUserInput): Promise<UserView> =>
    call<UserView>("create_user", { ...input }),
  listUsers: (): Promise<UserView[]> => call<UserView[]>("list_users"),
  getUserByCode: (code: string): Promise<UserView> =>
    call<UserView>("get_user_by_code", { code }),
  getCurrentUser: (): Promise<UserView> =>
    call<UserView>("current_user"),
  updateUser: (code: string, body: UpdateUserBody): Promise<UserView> =>
    call<UserView>("update_user", { code, body: { ...body } }),

  // product
  // (Product CRUD was removed from the server alongside the retired
  //  Product aggregate; the desktop wrappers were dropped with it.)

  // project
  createProject: (input: CreateProjectInput): Promise<ProjectView> =>
    call<ProjectView>("create_project", { ...input }),
  listProjects: (): Promise<ProjectView[]> => call<ProjectView[]>("list_projects"),
  getProjectByCode: (code: string): Promise<ProjectView> =>
    call<ProjectView>("get_project_by_code", { code }),
  updateProject: (code: string, body: UpdateProjectBody): Promise<ProjectView> =>
    call<ProjectView>("update_project", { code, body: { ...body } }),

  // health
  healthz: (): Promise<string> => call<string>("healthz"),

  // webview console forwarder — sink for `console.warn` /
  // `console.error` and the corresponding window error events.
  // The Rust command emits through tracing under the
  // `aegis_desktop_lib::webview` target with no trace id.
  forwardWebviewLog: (
    level: WebviewLogLevel,
    message: string,
  ): Promise<void> =>
    call<void>("forward_webview_log", { level, message }),

  // workspace window
  openProjectWorkspace: async (code: string): Promise<void> => {
    const label = `project:${code}`;
    const existing = await WebviewWindow.getByLabel(label);
    if (existing) {
      await existing.show();
      await existing.setFocus();
      return;
    }
    new WebviewWindow(label, {
      url: `/project/${code}`,
      title: code,
      width: 1100,
      height: 720,
      minWidth: 720,
      minHeight: 480,
      maximized: true,
    });
  },

  // terminology
  listTerminologyVersions: (): Promise<TerminologyVersionView[]> =>
    call<TerminologyVersionView[]>("list_terminology_versions"),
  createTerminologyVersion: (
    input: CreateTerminologyVersionInput,
  ): Promise<TerminologyVersionView> =>
    call<TerminologyVersionView>("create_terminology_version", { ...input }),
  getTerminologyVersionById: (id: number): Promise<TerminologyVersionView> =>
    call<TerminologyVersionView>("get_terminology_version_by_id", { id }),
  updateTerminologyVersion: (
    id: number,
    body: UpdateTerminologyVersionInput,
  ): Promise<TerminologyVersionView> =>
    call<TerminologyVersionView>("update_terminology_version", {
      id,
      body: { ...body },
    }),
  deleteTerminologyVersion: (id: number): Promise<void> =>
    call<void>("delete_terminology_version", { id }),

  listCodeLists: (
    versionId: number,
    options: CodeListListQuery = {},
  ): Promise<PagedCodeListListResponse> =>
    call<PagedCodeListListResponse>("list_code_lists", {
      versionId,
      fragment: options.fragment,
      offset: options.offset,
      limit: options.limit,
    }),
  getCodeListById: (id: number): Promise<CodeListView> =>
    call<CodeListView>("get_code_list_by_id", { id }),
  createCodeList: (input: CreateCodeListInput): Promise<CodeListView> =>
    call<CodeListView>("create_code_list", { ...input }),
  updateCodeList: (
    id: number,
    body: UpdateCodeListInput,
  ): Promise<CodeListView> =>
    call<CodeListView>("update_code_list", { id, body: { ...body } }),
  deleteCodeList: (id: number): Promise<void> =>
    call<void>("delete_code_list", { id }),

  listCodeItems: (
    codelistId: number | null,
    options: CodeItemListQuery & { versionId?: number | null } = {},
  ): Promise<PagedCodeItemListResponse> =>
    call<PagedCodeItemListResponse>("list_code_items", {
      codelistId: codelistId ?? undefined,
      versionId: options.versionId ?? undefined,
      fragment: options.fragment,
      offset: options.offset,
      limit: options.limit,
    }),
  listCodeItemsByVersionAndCode: (
    versionId: number,
    code: string,
  ): Promise<CodeItemListResponse> =>
    call<CodeItemListResponse>("list_code_items_by_version_and_code", {
      versionId,
      code,
    }),
  createCodeItem: (input: CreateCodeItemInput): Promise<CodeItemView> =>
    call<CodeItemView>("create_code_item", { ...input }),
  updateCodeItem: (
    id: number,
    body: UpdateCodeItemInput,
  ): Promise<CodeItemView> =>
    call<CodeItemView>("update_code_item", { id, body: { ...body } }),
  deleteCodeItem: (id: number): Promise<void> =>
    call<void>("delete_code_item", { id }),

  importTerminology: (
    kind: TerminologyKind,
    filepath: string,
  ): Promise<TerminologyVersionView> =>
    call<TerminologyVersionView>("import_terminology", { kind, filepath }),

  // domain-model
  listSdtmVersions: async (): Promise<SdtmVersionView[]> => {
    const resp = await call<SdtmVersionListResponse>("list_sdtm_versions");
    return resp.versions;
  },
  listSdtmDomainsByVersion: async (
    versionId: number,
  ): Promise<SdtmDomainView[]> => {
    const resp = await call<SdtmDomainListResponse>(
      "list_sdtm_domains_by_version",
      { versionId },
    );
    return resp.domains;
  },
  deleteSdtmDomain: (id: number): Promise<void> =>
    call<void>("delete_sdtm_domain", { id }),

  createSdtmDomain: (
    input: CreateSdtmDomainInput,
  ): Promise<SdtmDomainView> =>
    call<SdtmDomainView>("create_sdtm_domain", { input: { ...input } }),

  // SDTM domains (detail page)
  getSdtmDomainById: (id: number): Promise<SdtmDomainView> =>
    call<SdtmDomainView>("get_sdtm_domain_by_id", { id }),

  updateSdtmDomain: (
    id: number,
    body: UpdateSdtmDomainInput,
  ): Promise<SdtmDomainView> =>
    call<SdtmDomainView>("update_sdtm_domain", { id, body: { ...body } }),

  // SDTM variables
  listSdtmVariablesByDomain: async (
    domainId: number,
  ): Promise<SdtmVariableView[]> => {
    const resp = await call<SdtmVariableListResponse>(
      "list_sdtm_variables_by_domain",
      { domainId },
    );
    return resp.variables;
  },

  createSdtmVariable: (
    input: CreateSdtmVariableInput,
  ): Promise<SdtmVariableView> =>
    // The Rust command's struct payload parameter is named `input`; the
    // wrapper must match that key or Tauri rejects the call with
    // "command create_sdtm_variable missing required key input".
    call<SdtmVariableView>("create_sdtm_variable", { input: { ...input } }),

  updateSdtmVariable: (
    id: number,
    body: UpdateSdtmVariableInput,
  ): Promise<SdtmVariableView> =>
    call<SdtmVariableView>("update_sdtm_variable", { id, body: { ...body } }),

  deleteSdtmVariable: (id: number): Promise<void> =>
    call<void>("delete_sdtm_variable", { id }),

  // CRF
  listCrfVersions: async (projectCode: string): Promise<CrfVersion[]> => {
    const resp = await call<CrfVersionListResponse>("list_crf_versions", {
      projectCode,
    });
    return resp.versions;
  },
  listCrfFormsByVersion: async (versionId: number): Promise<CrfForm[]> => {
    const resp = await call<CrfFormListResponse>(
      "list_crf_forms_by_version",
      { versionId },
    );
    return resp.forms;
  },
  getCrfFormById: (id: number): Promise<CrfForm> =>
    call<CrfForm>("get_crf_form_by_id", { id }),
  createCrfForm: (
    versionId: number,
    body: CreateCrfFormInput,
  ): Promise<CrfForm> =>
    call<CrfForm>("create_crf_form", { versionId, body: { ...body } }),
  importAls: (
    name: string,
    projectCode: string,
    filepath: string,
    edcType: CrfEdcType,
  ): Promise<CrfVersion> =>
    call<CrfVersion>("import_als", { name, projectCode, filepath, edcType }),
  updateCrfForm: (
    id: number,
    body: UpdateCrfFormInput,
  ): Promise<CrfForm> =>
    call<CrfForm>("update_crf_form", { id, body: { ...body } }),
  deleteCrfForm: (id: number): Promise<void> =>
    call<void>("delete_crf_form", { id }),
  setCrfFormApproved: (
    id: number,
    approved: boolean,
    missionId: number,
  ): Promise<CrfForm> =>
    call<CrfForm>("set_crf_form_approved", { id, approved, missionId }),
  getCrfFormDetails: (id: number): Promise<CrfFormDetail> =>
    call<CrfFormDetail>("get_crf_form_details", { id }),
  searchCrfFormsByVersion: async (
    versionId: number,
    fragment: string,
  ): Promise<CrfForm[]> => {
    const resp = await call<CrfFormListResponse>(
      "search_crf_forms_by_version",
      { versionId, fragment },
    );
    return resp.forms;
  },
  searchCrfItemsByVersion: async (
    versionId: number,
    fragment: string,
  ): Promise<CrfItem[]> => {
    const resp = await call<{ items: CrfItem[] }>(
      "search_crf_items_by_version",
      { versionId, fragment },
    );
    return resp.items;
  },
  searchCrfOptionsByVersion: async (
    versionId: number,
    fragment: string,
  ): Promise<CrfOption[]> => {
    const resp = await call<CrfOptionListResponse>(
      "search_crf_options_by_version",
      { versionId, fragment },
    );
    return resp.options;
  },
  searchCrfUnitsByVersion: async (
    versionId: number,
    fragment: string,
  ): Promise<CrfUnit[]> => {
    const resp = await call<CrfUnitListResponse>(
      "search_crf_units_by_version",
      { versionId, fragment },
    );
    return resp.units;
  },
  searchCrfDomainAnnotationsByVersion: async (
    versionId: number,
    fragment: string,
  ): Promise<DomainAnnotation[]> => {
    const resp = await call<DomainAnnotationListResponse>(
      "search_crf_domain_annotations_by_version",
      { versionId, fragment },
    );
    return resp.domainAnnotations;
  },
  searchCrfAnnotationsByVersion: async (
    versionId: number,
    fragment: string,
  ): Promise<Annotation[]> => {
    const resp = await call<AnnotationListResponse>(
      "search_crf_annotations_by_version",
      { versionId, fragment },
    );
    return resp.annotations;
  },
  getCrfItemById: (id: number): Promise<CrfItem> =>
    call<CrfItem>("get_crf_item_by_id", { id }),
  getCrfOptionById: (id: number): Promise<CrfOption> =>
    call<CrfOption>("get_crf_option_by_id", { id }),
  getCrfUnitById: (id: number): Promise<CrfUnit> =>
    call<CrfUnit>("get_crf_unit_by_id", { id }),
  updateCrfItem: (
    id: number,
    body: UpdateCrfItemInput,
  ): Promise<CrfItem> =>
    call<CrfItem>("update_crf_item", { id, body: { ...body } }),
  updateCrfOption: (
    id: number,
    body: UpdateCrfOptionInput,
  ): Promise<CrfOption> =>
    call<CrfOption>("update_crf_option", { id, body: { ...body } }),
  updateCrfUnit: (
    id: number,
    body: UpdateCrfUnitInput,
  ): Promise<CrfUnit> =>
    call<CrfUnit>("update_crf_unit", { id, body: { ...body } }),
  listCrfItemsByForm: async (formId: number): Promise<CrfItem[]> => {
    const resp = await call<{ items: CrfItem[] }>("list_crf_items_by_form", {
      formId,
    });
    return resp.items;
  },
  createCrfDomainAnnotation: (
    formId: number,
    body: CreateDomainAnnotationInput,
  ): Promise<DomainAnnotation> =>
    call<DomainAnnotation>("create_crf_domain_annotation", {
      formId,
      body: { ...body },
    }),
  updateCrfDomainAnnotation: (
    id: number,
    body: UpdateDomainAnnotationInput,
  ): Promise<DomainAnnotation> =>
    call<DomainAnnotation>("update_crf_domain_annotation", {
      id,
      body: { ...body },
    }),
  deleteCrfDomainAnnotation: (id: number): Promise<void> =>
    call<void>("delete_crf_domain_annotation", { id }),
  createCrfAnnotation: (body: CreateAnnotationInput): Promise<Annotation> =>
    call<Annotation>("create_crf_annotation", { body: { ...body } }),
  updateCrfAnnotation: (
    id: number,
    body: UpdateAnnotationInput,
  ): Promise<Annotation> =>
    call<Annotation>("update_crf_annotation", { id, body: { ...body } }),
  deleteCrfAnnotation: (id: number): Promise<void> =>
    call<void>("delete_crf_annotation", { id }),

  // mission
  listMissionsByProject: (
    projectCode: string,
    kind?: MissionKind,
  ): Promise<MissionViewResponse[]> =>
    // The *server* returns a `MissionListResponse { missions: [...] }`
    // envelope, but `http::mission::list_by_project` already unwraps it,
    // so the Tauri command hands back a bare `Vec<MissionViewResponse>`.
    // Do not unwrap again here — `.missions` on an array is `undefined`.
    call<MissionViewResponse[]>("list_missions_by_project", {
      projectCode,
      kind,
    }),
  addAssignee: (
    missionId: number,
    body: AssigneeDataArg,
  ): Promise<AssigneeViewResponse> =>
    call<AssigneeViewResponse>("add_assignee", {
      missionId,
      body: { ...body },
    }),
  removeAssignee: (missionId: number, assigneeId: number): Promise<void> =>
    call<void>("remove_assignee", { missionId, assigneeId }),
  createMission: (input: CreateMissionInput): Promise<MissionViewResponse> =>
    call<MissionViewResponse>("create_mission", { ...input }),

  // mission issues
  listIssuesByMission: (
    missionId: number,
    options: IssueListQuery = {},
  ): Promise<IssueViewResponse[]> =>
    call<IssueViewResponse[]>("list_issues_by_mission", {
      missionId,
      state: options.state,
    }),
  createIssue: (
    missionId: number,
    body: CreateIssueInput,
  ): Promise<IssueViewResponse> =>
    call<IssueViewResponse>("create_issue", { missionId, body: { ...body } }),
  patchIssueState: (
    issueId: number,
    state: IssueState,
  ): Promise<IssueViewResponse> =>
    call<IssueViewResponse>("patch_issue_state", { issueId, state }),
  appendComment: (
    issueId: number,
    body: AppendCommentInput,
  ): Promise<IssueViewResponse> =>
    call<IssueViewResponse>("append_comment", { issueId, body: { ...body } }),

  // structures
  //
  // Tauri v2 resolves each command argument by the lower-camel-case of
  // the Rust parameter name and looks that key up in the invoke payload
  // with no whole-payload fallback. The Rust commands take their node
  // payload as a struct parameter named `body`, so the wrapper must
  // nest it under that key — flattening it is rejected at runtime with
  // "command add_structure_node missing required key body".
  // The key list is pinned by src-tauri/src/structures.contract.json and
  // checked by src/test/features/templates/api.test.ts.
  listStructures: (level?: StructureLevel): Promise<StructureSummaryView[]> =>
    call<StructureSummaryView[]>("list_structures", { level }),
  getStructure: (id: number): Promise<StructureView> =>
    call<StructureView>("get_structure", { id }),
  addStructureNode: (
    structureId: number,
    input: StructureNodeInput,
  ): Promise<StructureNodeView> =>
    call<StructureNodeView>("add_structure_node", {
      structureId,
      body: { ...input },
    }),
  updateStructureNode: (
    structureId: number,
    nodeId: string,
    input: StructureNodeInput,
  ): Promise<StructureNodeView> =>
    call<StructureNodeView>("update_structure_node", {
      structureId,
      nodeId,
      body: { ...input },
    }),
  deleteStructureNode: (structureId: number, nodeId: string): Promise<void> =>
    call<void>("delete_structure_node", { structureId, nodeId }),
} as const;

export type { ApiError } from "./types";
export type {
  Annotation,
  AppendCommentInput,
  AssigneeDataArg,
  AssigneeViewResponse,
  AnnotationOwner,
  CodeItemListQuery,
  CodeItemListResponse,
  CodeItemView,
  CodeListListQuery,
  CodeListListResponse,
  CodeListView,
  CreateAnnotationInput,
  CreateCodeItemInput,
  CreateCodeListInput,
  CreateIssueInput,
  CreateCrfFormInput,
  CreateDomainAnnotationInput,
  CreateMissionInput,
  CreateProjectInput,
  CreateSdtmDomainInput,
  CreateSdtmVariableInput,
  CreateTerminologyVersionInput,
  CreateUserInput,
  CrfEdcType,
  CrfForm,
  CrfFormDetail,
  CrfFormListResponse,
  CrfItem,
  CrfItemDetail,
  CrfItemKind,
  CrfOption,
  CrfOptionDetail,
  CrfUnit,
  CrfUnitDetail,
  CrfVersion,
  CrfVersionListResponse,
  DomainAnnotation,
  DomainCategory,
  Identity,
  ImportAlsInput,
  IssueCommentViewResponse,
  IssueListQuery,
  IssueListResponse,
  IssueState,
  IssueViewResponse,
  MigrationView,
  MissionKind,
  MissionRole,
  MissionViewResponse,
  PagedCodeItemListResponse,
  PagedCodeListListResponse,
  ProjectMembers,
  ProjectMembersView,
  ProjectConfiguration,
  ProjectConfigurationSdtmig,
  ProjectConfigurationTerminology,
  ProjectLanguage,
  ProjectView,
  Role,
  RegisterUserInput,
  RegisterUserResponse,
  SdtmDomainDescription,
  SdtmDomainDescriptionDetail,
  SdtmDomainListResponse,
  SdtmDomainView,
  SdtmRole,
  SdtmVariableCore,
  SdtmVariableControlled,
  SdtmVariableDescription,
  SdtmVariableDescriptionDetail,
  SdtmVariableListResponse,
  SdtmVariableType,
  SdtmVariableView,
  SdtmVersionListResponse,
  SdtmVersionView,
  StructureLevel,
  StructureNodeInput,
  StructureNodeView,
  StructureSummaryView,
  StructureView,
  Tag,
  TerminologyKind,
  TerminologyVersionListResponse,
  TerminologyVersionView,
  UpdateAnnotationInput,
  UpdateCodeItemInput,
  UpdateCodeListInput,
  UpdateCrfFormInput,
  UpdateCrfItemInput,
  UpdateCrfOptionInput,
  UpdateCrfUnitInput,
  UpdateDomainAnnotationInput,
  UpdateProjectBody,
  UpdateSdtmDomainInput,
  UpdateSdtmVariableInput,
  UpdateTerminologyVersionInput,
  UpdateUserBody,
  UpdateUserCredentialInput,
  UserCredentialView,
  UserSummary,
  UserView,
  WebviewLogLevel,
} from "./types";
