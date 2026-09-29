WITH RankedPosts AS (
    SELECT 
        p.Id AS PostId,
        p.Title,
        p.CreationDate,
        p.Score,
        p.ViewCount,
        p.AnswerCount,
        p.CommentCount,
        u.DisplayName AS OwnerDisplayName,
        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC, p.Id) AS Rank
    FROM 
        Posts p
    JOIN 
        Users u ON p.OwnerUserId = u.Id
    WHERE 
        p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '90 days'
), 
TopVotedPosts AS (
    SELECT 
        rp.*, 
        vt.Name AS VoteTypeName
    FROM 
        RankedPosts rp
    LEFT JOIN 
        Votes v ON rp.PostId = v.PostId
    LEFT JOIN 
        VoteTypes vt ON v.VoteTypeId = vt.Id
    WHERE 
        rp.Rank <= 10
), 
PostWithLatestComment AS (
    SELECT 
        tp.*, 
        c.Text AS LatestCommentText,
        c.CreationDate AS LatestCommentDate
    FROM 
        TopVotedPosts tp
    LEFT JOIN 
        Comments c ON tp.PostId = c.PostId
    WHERE 
        c.CreationDate = (SELECT MAX(CreationDate) FROM Comments WHERE PostId = tp.PostId)
)
SELECT 
    p.PostId,
    p.Title,
    p.OwnerDisplayName,
    p.Score,
    p.ViewCount,
    p.AnswerCount,
    p.CommentCount,
    p.VoteTypeName,
    p.LatestCommentText,
    p.LatestCommentDate
FROM 
    PostWithLatestComment p
ORDER BY 
    p.Score DESC, p.ViewCount DESC;