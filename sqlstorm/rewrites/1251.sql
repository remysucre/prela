WITH RankedPosts AS (
    SELECT 
        p.Id AS PostId,
        p.Title,
        p.CreationDate,
        p.ViewCount,
        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.ViewCount DESC, p.Id) AS Rank,
        u.Reputation AS UserReputation
    FROM 
        Posts p
    LEFT JOIN 
        Users u ON p.OwnerUserId = u.Id
    WHERE 
        p.CreationDate >= cast('2024-10-01' as date) - INTERVAL '1 year'
),
PostsWithComments AS (
    SELECT 
        r.PostId,
        r.Title,
        r.CreationDate,
        r.ViewCount,
        COALESCE(c.CommentCount, 0) AS CommentCount,
        r.UserReputation
    FROM 
        RankedPosts r
    LEFT JOIN 
        (SELECT PostId, COUNT(*) AS CommentCount 
         FROM Comments 
         GROUP BY PostId) c ON r.PostId = c.PostId
    WHERE 
        r.Rank <= 10
),
TopPosts AS (
    SELECT 
        p.*,
        CASE 
            WHEN p.UserReputation >= 1000 THEN 'Expert'
            WHEN p.UserReputation >= 100 THEN 'Contributor'
            ELSE 'Novice'
        END AS UserLevel
    FROM 
        PostsWithComments p
    WHERE 
        p.CommentCount > 5
)
SELECT 
    t.Title,
    t.CreationDate,
    t.ViewCount,
    t.CommentCount,
    t.UserLevel,
    CASE 
        WHEN t.UserLevel = 'Expert' THEN 'Highly recommended'
        WHEN t.UserLevel = 'Contributor' THEN 'Moderate recommendation'
        ELSE 'Not recommended'
    END AS Recommendation
FROM 
    TopPosts t
ORDER BY 
    t.ViewCount DESC;