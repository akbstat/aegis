-- sdtm_variables.variable_controlled changes from TEXT to JSONB
-- carrying a SdtmVariableControlled object ({"code": [...], "format": [...]}).
-- The previous TEXT value did not record whether it was a codelist or a
-- format, so it cannot be converted and is dropped to NULL.

ALTER TABLE sdtm_variables
    ALTER COLUMN variable_controlled TYPE JSONB
    USING NULL::jsonb;
