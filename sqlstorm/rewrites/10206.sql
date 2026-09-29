
WITH PostStats AS (
    SELECT 
        p.Id AS PostId,
        p.Title,
        p.CreationDate,
        p.Score,
        p.ViewCount,
        COUNT(c.Id) AS CommentCount,
        COUNT(v.Id) AS VoteCount,
        pt.Name AS PostType,
        u.DisplayName AS OwnerDisplayName
    FROM 
        Posts p
    LEFT JOIN 
        Comments c ON p.Id = c.PostId
    LEFT JOIN 
        Votes v ON p.Id = v.PostId
    LEFT JOIN 
        PostTypes pt ON p.PostTypeId = pt.Id
    LEFT JOIN 
        Users u ON p.OwnerUserId = u.Id
    GROUP BY 
        p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, pt.Name, u.DisplayName
),
TopPosts AS (
    SELECT 
        PostId,
        Title,
        CreationDate,
        Score,
        ViewCount,
        CommentCount,
        VoteCount,
        PostType,
        OwnerDisplayName,
        ROW_NUMBER() OVER (ORDER BY Score DESC, PostId) AS ScoreRank,
        ROW_NUMBER() OVER (ORDER BY ViewCount DESC, PostId) AS ViewRank
    FROM 
        PostStats
)
SELECT 
    PostId,
    Title,
    CreationDate,
    Score,
    ViewCount,
    CommentCount,
    VoteCount,
    PostType,
    OwnerDisplayName,
    ScoreRank,
    ViewRank
FROM 
    TopPosts
WHERE 
    ScoreRank <= 10 OR ViewRank <= 10
ORDER BY 
    Score DESC, ViewCount DESC;
