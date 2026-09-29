
WITH CTE_UserReputation AS (
    SELECT 
        U.Id AS UserId, 
        U.DisplayName,
        CASE 
            WHEN U.Reputation IS NULL THEN 'Unknown Reputation'
            WHEN U.Reputation > 1000 THEN 'High Reputation'
            ELSE 'Low Reputation' 
        END AS REPUTATION,
        P.ViewCount AS PostViewCount,
        ROW_NUMBER() OVER (PARTITION BY U.Id ORDER BY P.CreationDate DESC) AS PostRowNum
    FROM 
        Users U
    LEFT JOIN 
        Posts P ON U.Id = P.OwnerUserId
),
CTE_RecentPosts AS (
    SELECT 
        A.Id AS AnswerId, 
        A.OwnerUserId, 
        A.Title, 
        A.CreationDate, 
        A.AcceptedAnswerId,
        CASE 
            WHEN A.AcceptedAnswerId IS NULL THEN 'Not Accepted'
            ELSE 'Accepted' 
        END AS AcceptanceStatus,
        (SELECT COUNT(*) FROM Votes V WHERE V.PostId = A.Id AND V.VoteTypeId = 2) AS UpVoteCount,
        (SELECT COUNT(*) FROM Votes V WHERE V.PostId = A.Id AND V.VoteTypeId = 3) AS DownVoteCount
    FROM 
        Posts A
    WHERE 
        A.PostTypeId = 2 AND 
        A.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '30 days' 
),
CTE_PostLinks AS (
    SELECT 
        PL.PostId,
        PL.RelatedPostId,
        PL.LinkTypeId,
        CASE 
            WHEN PL.LinkTypeId = 1 THEN 'Standard Link'
            WHEN PL.LinkTypeId = 3 THEN 'Duplicate Link'
            ELSE 'Other Link Type' 
        END AS LinkTypeDescription
    FROM 
        PostLinks PL
),
CombinedData AS (
    SELECT 
        U.DisplayName,
        U.Reputation,
        R.AnswerId,
        R.Title AS RecentPostTitle,
        R.AcceptanceStatus,
        R.UpVoteCount,
        R.DownVoteCount,
        P.ViewCount AS MostViewedPost,
        L.LinkTypeDescription
    FROM 
        CTE_UserReputation U
    JOIN 
        CTE_RecentPosts R ON U.UserId = R.OwnerUserId
    JOIN 
        (SELECT 
            OwnerUserId, 
            MAX(ViewCount) AS ViewCount 
         FROM 
            Posts 
         GROUP BY 
            OwnerUserId) AS P ON U.UserId = P.OwnerUserId
    LEFT JOIN 
        CTE_PostLinks L ON R.AnswerId = L.PostId
    WHERE 
        U.Reputation IS NOT NULL
)
SELECT 
    DisplayName,
    Reputation,
    RecentPostTitle,
    AcceptanceStatus,
    UpVoteCount,
    DownVoteCount,
    MostViewedPost,
    LinkTypeDescription,
    CASE 
        WHEN UpVoteCount - DownVoteCount > 10 THEN 'Highly Liked'
        WHEN UpVoteCount - DownVoteCount < 0 THEN 'Disliked'
        ELSE 'Neutral'
    END AS PostSentiment
FROM 
    CombinedData
WHERE 
    (CASE 
        WHEN UpVoteCount - DownVoteCount > 10 THEN 'Highly Liked'
        WHEN UpVoteCount - DownVoteCount < 0 THEN 'Disliked'
        ELSE 'Neutral'
    END) IN ('Highly Liked', 'Disliked') 
ORDER BY 
    Reputation DESC, UpVoteCount DESC, AnswerId, LinkTypeDescription
FETCH FIRST 100 ROWS ONLY;
