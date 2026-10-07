ENTITIES = {
    "Patient": ("mimiciv_hosp.patients", "subject_id"),
    "Admission": ("mimiciv_hosp.admissions", "hadm_id"),
    "IcuStay": ("mimiciv_icu.icustays", "stay_id"),
    "DiagnosesIcd": ("mimiciv_hosp.diagnoses_icd", None),
    "LabEvent": ("mimiciv_hosp.labevents", None),
    "MicrobiologyEvent": ("mimiciv_hosp.microbiologyevents", None),
    "Prescription": ("mimiciv_hosp.prescriptions", None),
    "Service": ("mimiciv_hosp.services", None),
    "ChartEvent": ("mimiciv_icu.chartevents", None),
    "InputEvent": ("mimiciv_icu.inputevents", None),
    "OutputEvent": ("mimiciv_icu.outputevents", None),
    "ProcedureEvent": ("mimiciv_icu.procedureevents", None),
}

FK_COLUMNS = {
    "subject_id": "mimiciv_hosp.patients",
    "hadm_id": "mimiciv_hosp.admissions",
    "stay_id": "mimiciv_icu.icustays",
}
