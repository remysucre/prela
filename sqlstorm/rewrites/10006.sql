
WITH UserPostSummary AS (
    SELECT 
        U.Id AS UserId,
        U.DisplayName,
        COUNT(P.Id) AS TotalPosts,
        SUM(CASE WHEN P.PostTypeId = 1 THEN 1 ELSE 0 END) AS TotalQuestions,
        SUM(CASE WHEN P.PostTypeId = 2 THEN 1 ELSE 0 END) AS TotalAnswers,
        SUM(P.Score) AS TotalScore,
        SUM(P.ViewCount) AS TotalViews,
        SUM(epoch_us(P.CreationDate))::DOUBLE / COUNT(P.CreationDate) / 1e6 AS AveragePostCreationDate
    FROM 
        Users U
    LEFT JOIN 
        Posts P ON U.Id = P.OwnerUserId
    GROUP BY 
        U.Id, U.DisplayName
), 
PostActivitySummary AS (
    SELECT 
        P.Id AS PostId,
        P.Title,
        P.CreationDate,
        P.Score,
        COALESCE(C.Count, 0) AS CommentCount,
        COALESCE(V.VoteCount, 0) AS VoteCount,
        P.OwnerUserId
    FROM 
        Posts P
    LEFT JOIN (
        SELECT PostId, COUNT(*) AS Count
        FROM Comments
        GROUP BY PostId
    ) C ON P.Id = C.PostId
    LEFT JOIN (
        SELECT PostId, COUNT(*) AS VoteCount
        FROM Votes
        GROUP BY PostId
    ) V ON P.Id = V.PostId
)
SELECT 
    U.UserId,
    U.DisplayName,
    U.TotalPosts,
    U.TotalQuestions,
    U.TotalAnswers,
    U.TotalScore,
    U.TotalViews,
    U.AveragePostCreationDate,
    P.PostId,
    P.Title,
    P.CreationDate,
    P.Score,
    P.CommentCount,
    P.VoteCount
FROM 
    UserPostSummary U
JOIN 
    PostActivitySummary P ON U.UserId = P.OwnerUserId
ORDER BY 
    U.TotalPosts DESC, U.TotalScore DESC, P.Score DESC;
