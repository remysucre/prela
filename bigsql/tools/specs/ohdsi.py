ENTITIES = {
    "Person": ("main.person", "PERSON_ID"),
    "Concept": ("main.concept", "CONCEPT_ID"),
    "VisitOccurrence": ("main.visit_occurrence", "VISIT_OCCURRENCE_ID"),
    "ObservationPeriod": ("main.observation_period", "OBSERVATION_PERIOD_ID"),
    "ConceptAncestor": ("main.concept_ancestor", None),
    "ConditionOccurrence": ("main.condition_occurrence", "CONDITION_OCCURRENCE_ID"),
    "DrugExposure": ("main.drug_exposure", "DRUG_EXPOSURE_ID"),
    "ProcedureOccurrence": ("main.procedure_occurrence", "PROCEDURE_OCCURRENCE_ID"),
    "Observation": ("main.observation", "OBSERVATION_ID"),
    "Measurement": ("main.measurement", "MEASUREMENT_ID"),
}

FK_COLUMNS = {
    "person_id": "main.person",
    "visit_occurrence_id": "main.visit_occurrence",
    "ancestor_concept_id": "main.concept",
    "descendant_concept_id": "main.concept",
}

FK_SUFFIX = {"_concept_id": "main.concept"}
