WITH RankedPosts AS (
    SELECT 
        P.Id AS PostId,
        P.Title,
        P.CreationDate,
        P.Score,
        P.ViewCount,
        U.DisplayName AS OwnerName,
        ROW_NUMBER() OVER (PARTITION BY P.OwnerUserId ORDER BY P.CreationDate DESC) AS PostRank
    FROM 
        Posts P
    JOIN 
        Users U ON P.OwnerUserId = U.Id
    WHERE 
        P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
),
TopRankedPosts AS (
    SELECT 
        PostId,
        Title,
        CreationDate,
        Score,
        ViewCount,
        OwnerName
    FROM 
        RankedPosts
    WHERE 
        PostRank = 1
),
PostInteractionStats AS (
    SELECT 
        P.Id AS PostId,
        COALESCE(COUNT(V.Id), 0) AS VoteCount,
        COALESCE(COUNT(C.Id), 0) AS CommentCount
    FROM 
        Posts P
    LEFT JOIN 
        Votes V ON P.Id = V.PostId
    LEFT JOIN 
        Comments C ON P.Id = C.PostId
    GROUP BY 
        P.Id
),
FinalPostStats AS (
    SELECT 
        TRP.PostId,
        TRP.Title,
        TRP.CreationDate,
        TRP.Score + PIS.VoteCount AS TotalScore,
        PIS.CommentCount
    FROM 
        TopRankedPosts TRP
    JOIN 
        PostInteractionStats PIS ON TRP.PostId = PIS.PostId
)
SELECT 
    FPS.PostId,
    FPS.Title,
    FPS.CreationDate,
    FPS.TotalScore,
    FPS.CommentCount,
    CASE 
        WHEN FPS.TotalScore > 100 THEN 'Highly Engaging'
        WHEN FPS.TotalScore BETWEEN 50 AND 100 THEN 'Moderately Engaging'
        ELSE 'Less Engaging'
    END AS EngagementLevel
FROM 
    FinalPostStats FPS
ORDER BY 
    FPS.TotalScore DESC, FPS.PostId
LIMIT 10;