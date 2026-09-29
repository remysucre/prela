
WITH PostStatistics AS (
    SELECT 
        P.Id AS PostId,
        P.Title,
        P.CreationDate,
        P.Score,
        COALESCE((SELECT COUNT(*) FROM Comments C WHERE C.PostId = P.Id), 0) AS CommentCount,
        COALESCE((SELECT COUNT(*) FROM Votes V WHERE V.PostId = P.Id AND V.VoteTypeId = 2), 0) AS Upvotes,
        COALESCE((SELECT COUNT(*) FROM Votes V WHERE V.PostId = P.Id AND V.VoteTypeId = 3), 0) AS Downvotes,
        DENSE_RANK() OVER (ORDER BY P.Score DESC) AS ScoreRank,
        ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC, P.Id) AS UserPostRank,
        P.OwnerUserId
    FROM 
        Posts P
    WHERE 
        P.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
),

UserBadges AS (
    SELECT 
        U.Id AS UserId,
        COUNT(B.Id) AS TotalBadges,
        MAX(B.Class) AS HighestBadgeClass
    FROM 
        Users U
    LEFT JOIN 
        Badges B ON U.Id = B.UserId
    GROUP BY 
        U.Id
)

SELECT 
    PS.PostId,
    PS.Title,
    PS.CreationDate,
    PS.Score,
    PS.CommentCount,
    PS.Upvotes,
    PS.Downvotes,
    PS.ScoreRank,
    UB.TotalBadges,
    UB.HighestBadgeClass,
    CASE 
        WHEN PS.Score > 10 THEN 'High'
        WHEN PS.Score BETWEEN 5 AND 10 THEN 'Medium'
        ELSE 'Low'
    END AS ScoreCategory,
    CASE 
        WHEN PS.UserPostRank = 1 THEN 'Most Recent Post'
        ELSE NULL
    END AS RecentPostIndicator
FROM 
    PostStatistics PS
LEFT JOIN 
    UserBadges UB ON PS.OwnerUserId = UB.UserId
ORDER BY 
    PS.Score DESC, 
    PS.CreationDate DESC
LIMIT 100;
