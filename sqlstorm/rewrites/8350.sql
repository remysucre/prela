
WITH RankedPosts AS (
    SELECT 
        p.Id AS PostId,
        p.Title,
        u.DisplayName AS OwnerDisplayName,
        p.CreationDate,
        p.Score,
        COUNT(c.Id) AS CommentCount,
        COUNT(DISTINCT v.Id) AS VoteCount,
        ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.CreationDate DESC) AS PostRank
    FROM 
        Posts p
        LEFT JOIN Users u ON p.OwnerUserId = u.Id
        LEFT JOIN Comments c ON p.Id = c.PostId
        LEFT JOIN Votes v ON p.Id = v.PostId
    WHERE 
        p.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year' 
        AND p.PostTypeId IN (1, 2) 
    GROUP BY 
        p.Id, p.Title, u.DisplayName, p.CreationDate, p.Score
),
PopularPosts AS (
    SELECT 
        PostId, 
        Title,
        OwnerDisplayName,
        CreationDate,
        Score,
        CommentCount,
        VoteCount
    FROM 
        RankedPosts
    WHERE 
        PostRank = 1
)
SELECT 
    pp.*,
    COALESCE(rvt.Name, 'No votes') AS MostRecentVoteType
FROM 
    PopularPosts pp
    LEFT JOIN Votes v ON pp.PostId = v.PostId
    LEFT JOIN VoteTypes rvt ON v.VoteTypeId = rvt.Id
WHERE 
    pp.VoteCount > 5
ORDER BY 
    pp.Score DESC, 
    pp.CommentCount DESC, pp.PostId, v.Id
LIMIT 10;
