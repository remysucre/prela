
WITH RECURSIVE UserEngagement AS (
    SELECT 
        U.Id AS UserId,
        U.DisplayName,
        COUNT(DISTINCT P.Id) AS TotalPosts,
        COALESCE(SUM(CASE WHEN V.VoteTypeId IN (2, 3) THEN 1 ELSE 0 END), 0) AS TotalVotes,
        COALESCE(SUM(CASE WHEN C.Id IS NOT NULL THEN 1 ELSE 0 END), 0) AS TotalComments,
        TIMESTAMP '2024-10-01 12:34:56' - U.CreationDate AS AccountAge
    FROM 
        Users U
    LEFT JOIN 
        Posts P ON U.Id = P.OwnerUserId
    LEFT JOIN 
        Votes V ON P.Id = V.PostId
    LEFT JOIN 
        Comments C ON P.Id = C.PostId
    GROUP BY 
        U.Id, U.DisplayName, U.CreationDate
),
UserPosts AS (
    SELECT 
        U.Id AS UserId,
        U.DisplayName,
        P.Id AS PostId,
        P.Title,
        P.CreationDate,
        P.Score,
        ROW_NUMBER() OVER(PARTITION BY U.Id ORDER BY P.CreationDate DESC, P.Id) AS PostRank
    FROM 
        Users U
    INNER JOIN 
        Posts P ON U.Id = P.OwnerUserId
),
TopEngagedUsers AS (
    SELECT 
        UserId,
        DisplayName,
        TotalPosts,
        TotalVotes,
        TotalComments,
        AccountAge,
        DENSE_RANK() OVER(ORDER BY TotalPosts DESC, TotalVotes DESC, TotalComments DESC) AS EngagementRank
    FROM 
        UserEngagement
    WHERE 
        TotalPosts > 0
)
SELECT 
    U.UserId,
    U.DisplayName,
    U.TotalPosts,
    U.TotalVotes,
    U.TotalComments,
    U.AccountAge,
    P.Title AS LatestPostTitle,
    P.CreationDate AS LatestPostDate,
    P.Score AS LatestPostScore,
    CASE 
        WHEN P.Score IS NULL THEN 'No posts yet'
        ELSE CASE 
            WHEN P.Score > 10 THEN 'High engagement'
            WHEN P.Score BETWEEN 1 AND 10 THEN 'Moderate engagement'
            ELSE 'Low engagement'
        END
    END AS EngagementLevel,
    (SELECT COUNT(*) FROM Badges B WHERE B.UserId = U.UserId) AS BadgeCount
FROM 
    TopEngagedUsers U
LEFT JOIN 
    UserPosts P ON U.UserId = P.UserId AND P.PostRank = 1
WHERE 
    U.EngagementRank <= 10
ORDER BY 
    U.EngagementRank;
