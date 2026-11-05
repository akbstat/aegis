import { useState } from "react";
import { Alert, Box, CircularProgress, Typography } from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";
import { useParams } from "@tanstack/react-router";

import { useProjectConfiguration } from "../data/project-configuration";
import { ConfigurationFilepathSection } from "../components/ConfigurationFilepathSection";
import { ConfigurationGeneralSection } from "../components/ConfigurationGeneralSection";
import { ConfigurationMembersSection } from "../components/ConfigurationMembersSection";
import { ProjectBranchesSection } from "../components/ProjectBranchesSection";
import {
  ConfigurationSidebar,
  type ConfigurationSection,
} from "../components/ConfigurationSidebar";
import { errorMessage } from "../../../shared/api/error";

/**
 * Project configuration page. Right-side nav picks between
 * General / Members / Filepath. Project leaders can edit; everyone
 * else sees the controls in read-only mode (no Save buttons).
 *
 * The leader flag comes from `useProjectConfiguration`: `null`
 * while either the user or the project query is still loading
 * (so we don't briefly flash the read-only banner). Once it
 * settles to `false`, the read-only banner appears and every
 * section's Save button is suppressed by the `readonly` prop.
 */
export function ProjectConfigurationPage() {
  const { t } = useI18n();
  const { projectCode } = useParams({ strict: false }) as {
    projectCode: string;
  };
  const [section, setSection] = useState<ConfigurationSection>("general");
  const { data, isLeader, isLoading, isError, error } =
    useProjectConfiguration(projectCode);

  if (isLoading) {
    return (
      <Box
        sx={{
          display: "flex",
          justifyContent: "center",
          alignItems: "center",
          minHeight: "50vh",
        }}
        data-testid="config-loading"
      >
        <CircularProgress />
      </Box>
    );
  }

  if (isError || !data) {
    return (
      <Box sx={{ p: 4 }}>
        <Alert severity="error">
          {t("project.loadFailed", {
            message: error ? errorMessage(error) : "",
          })}
        </Alert>
      </Box>
    );
  }

  const readonly = isLeader !== true;

  return (
    <Box sx={{ display: "flex", minHeight: "100vh" }}>
      <Box
        component="main"
        sx={{
          flexGrow: 1,
          p: 4,
          display: "flex",
          flexDirection: "column",
          gap: 3,
        }}
      >
        <Typography variant="h4" gutterBottom>
          {t("project.configuration.heading")}
        </Typography>
        {readonly && (
          <Alert severity="info" data-testid="config-readonly">
            {t("project.configuration.readOnly")}
          </Alert>
        )}

        {section === "general" && (
          <ConfigurationGeneralSection
            projectCode={projectCode}
            readonly={readonly}
            initial={data.configurations}
          />
        )}
        {section === "members" && (
          <ConfigurationMembersSection
            projectCode={projectCode}
            readonly={readonly}
            initial={data.members}
          />
        )}
        {section === "branches" && (
          <ProjectBranchesSection
            projectCode={projectCode}
            isLeader={isLeader === true}
          />
        )}
        {section === "filepath" && <ConfigurationFilepathSection />}
      </Box>
      <ConfigurationSidebar value={section} onChange={setSection} />
    </Box>
  );
}