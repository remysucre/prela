WITH RankedPosts AS (
    SELECT 
        p.Id AS PostId,
        p.Title,
        p.CreationDate,
        p.ViewCount,
        p.Score,
        u.DisplayName AS OwnerDisplayName,
        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.ViewCount DESC, p.Id) AS Rank
    FROM 
        Posts p
    JOIN 
        Users u ON p.OwnerUserId = u.Id
    WHERE 
        p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '6 months'
),
TopPosts AS (
    SELECT 
        PostId, 
        Title, 
        CreationDate, 
        ViewCount, 
        Score, 
        OwnerDisplayName
    FROM 
        RankedPosts
    WHERE 
        Rank <= 5
),
PostDetails AS (
    SELECT 
        tp.*,
        COALESCE(c.CommentCount, 0) AS TotalComments,
        COALESCE(v.UpVoteCount, 0) AS TotalUpVotes
    FROM 
        TopPosts tp
    LEFT JOIN (
        SELECT 
            PostId, 
            COUNT(*) AS CommentCount 
        FROM 
            Comments 
        GROUP BY 
            PostId
    ) c ON tp.PostId = c.PostId
    LEFT JOIN (
        SELECT 
            PostId, 
            COUNT(*) FILTER (WHERE VoteTypeId = 2) AS UpVoteCount 
        FROM 
            Votes 
        GROUP BY 
            PostId
    ) v ON tp.PostId = v.PostId
)

SELECT 
    pd.PostId,
    pd.Title,
    pd.CreationDate,
    pd.ViewCount,
    pd.Score,
    pd.TotalComments,
    pd.TotalUpVotes,
    coalesce(h.ResponsesCount, 0) AS TotalResponses,
    CASE 
        WHEN pd.Score > 10 THEN 'Hot'
        WHEN pd.ViewCount > 1000 THEN 'Trending'
        ELSE 'Regular'
    END AS PostCategory
FROM 
    PostDetails pd
LEFT JOIN (
    SELECT 
        p.Id AS PostId,
        COUNT(*) AS ResponsesCount
    FROM 
        Posts p 
    WHERE 
        p.PostTypeId = 2 
        AND p.ParentId IN (SELECT PostId FROM PostDetails)
    GROUP BY 
        p.Id
) h ON pd.PostId = h.PostId
ORDER BY 
    pd.Score DESC, 
    pd.ViewCount DESC;