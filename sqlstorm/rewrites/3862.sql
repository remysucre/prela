
WITH RankedPosts AS (
    SELECT 
        P.Id AS PostId,
        P.Title,
        P.CreationDate,
        P.Score,
        P.ViewCount,
        ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.Score DESC, P.Id) AS PostRank
    FROM 
        Posts P
    WHERE 
        P.PostTypeId = 1
),
UserStats AS (
    SELECT 
        U.Id AS UserId,
        U.Reputation,
        U.DisplayName,
        COUNT(DISTINCT P.Id) AS QuestionCount,
        SUM(COALESCE(P.ViewCount, 0)) AS TotalViewCount,
        SUM(COALESCE(P.Score, 0)) AS TotalScore
    FROM 
        Users U
    LEFT JOIN 
        Posts P ON U.Id = P.OwnerUserId AND P.PostTypeId = 1
    GROUP BY 
        U.Id, U.Reputation, U.DisplayName
),
AnswerDetails AS (
    SELECT
        A.ParentId AS QuestionId,
        COUNT(A.Id) AS AnswerCount,
        SUM(COALESCE(A.Score, 0)) AS TotalAnswerScore
    FROM 
        Posts A
    WHERE 
        A.PostTypeId = 2
    GROUP BY 
        A.ParentId
)
SELECT 
    U.DisplayName,
    U.Reputation,
    U.QuestionCount,
    U.TotalViewCount,
    U.TotalScore,
    COALESCE(AD.AnswerCount, 0) AS AnswerCount,
    COALESCE(AD.TotalAnswerScore, 0) AS TotalAnswerScore,
    RP.Title AS TopPostTitle,
    RP.CreationDate AS TopPostDate,
    RP.Score AS TopPostScore
FROM 
    UserStats U
LEFT JOIN 
    AnswerDetails AD ON U.UserId = AD.QuestionId
LEFT JOIN 
    RankedPosts RP ON U.UserId = RP.PostId
WHERE 
    RP.PostRank = 1
ORDER BY 
    U.Reputation DESC, 
    U.QuestionCount DESC,
U.UserId
FETCH FIRST 50 ROWS ONLY;
