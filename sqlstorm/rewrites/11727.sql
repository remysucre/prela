
WITH UserPostStats AS (
    SELECT 
        U.Id AS UserId,
        U.DisplayName,
        COUNT(P.Id) AS PostCount,
        SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS QuestionCount,
        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS AnswerCount,
        SUM(P.ViewCount) AS TotalViews,
        SUM(P.Score) AS TotalScore,
        SUM(epoch_us(P.CreationDate))::DOUBLE / COUNT(P.CreationDate) / 1e6 AS AvgPostAge
    FROM 
        Users U
    LEFT JOIN 
        Posts P ON U.Id = P.OwnerUserId
    GROUP BY 
        U.Id, U.DisplayName
),
PostFeatureStats AS (
    SELECT 
        P.Id AS PostId,
        P.Title,
        P.CreationDate,
        P.ViewCount,
        P.Score,
        C.CommentCount,
        PH.RevisionCount,
        P.OwnerUserId
    FROM 
        Posts P
    LEFT JOIN 
        (SELECT PostId, COUNT(*) AS CommentCount FROM Comments GROUP BY PostId) C ON P.Id = C.PostId
    LEFT JOIN 
        (SELECT PostId, COUNT(*) AS RevisionCount FROM PostHistory GROUP BY PostId) PH ON P.Id = PH.PostId
)
SELECT 
    U.UserId,
    U.DisplayName,
    U.PostCount,
    U.QuestionCount,
    U.AnswerCount,
    U.TotalViews,
    U.TotalScore,
    U.AvgPostAge,
    PFS.PostId,
    PFS.Title,
    PFS.CreationDate,
    PFS.ViewCount,
    PFS.Score,
    PFS.CommentCount,
    PFS.RevisionCount
FROM 
    UserPostStats U
JOIN 
    PostFeatureStats PFS ON U.UserId = PFS.OwnerUserId
ORDER BY 
    U.PostCount DESC, U.TotalScore DESC;
