import { useNavigate } from "@tanstack/react-router";
import { Box, Card, CardContent, CardHeader, Typography } from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

/**
 * The template catalogue.
 *
 * The entries are fixed text in the frontend — they are not fetched —
 * so each card is a launcher into one of the template pages rather than
 * a row of API data.
 */
const TEMPLATES = [
  {
    titleKey: "templates.card.standardFolderStructure.title",
    descriptionKey: "templates.card.standardFolderStructure.description",
    target: "/templates/structure",
  },
] as const;

export function TemplatesPage() {
  const { t } = useI18n();
  const navigate = useNavigate();

  return (
    <Box sx={{ p: 4, display: "flex", flexDirection: "column", gap: 3 }}>
      <Typography variant="h4">{t("templates.heading")}</Typography>

      <Box sx={{ display: "flex", gap: 3, flexWrap: "wrap" }}>
        {TEMPLATES.map((template) => (
          <Card
            key={template.target}
            sx={{ flex: 1, minWidth: 320, cursor: "pointer" }}
            onClick={() => navigate({ to: template.target })}
          >
            <CardHeader title={t(template.titleKey)} />
            <CardContent>
              <Typography variant="body2">
                {t(template.descriptionKey)}
              </Typography>
            </CardContent>
          </Card>
        ))}
      </Box>
    </Box>
  );
}
