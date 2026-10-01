
WITH RankedPosts AS (
    SELECT 
        p.Id AS PostId,
        p.Title,
        p.Tags,
        p.CreationDate,
        p.OwnerUserId,
        u.DisplayName AS OwnerDisplayName,
        COUNT(a.Id) AS AnswerCount,
        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
        ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.CreationDate DESC) AS rn
    FROM 
        Posts p
    LEFT JOIN 
        Users u ON p.OwnerUserId = u.Id
    LEFT JOIN 
        Posts a ON a.ParentId = p.Id
    LEFT JOIN 
        Votes v ON v.PostId = p.Id
    WHERE 
        p.PostTypeId = 1  
    GROUP BY 
        p.Id, p.Title, p.Tags, p.CreationDate, p.OwnerUserId, u.DisplayName
), FilteredPosts AS (
    SELECT 
        rp.PostId,
        rp.Title,
        rp.Tags,
        rp.CreationDate,
        rp.OwnerUserId,
        rp.OwnerDisplayName,
        rp.AnswerCount,
        rp.UpVotes,
        rp.DownVotes
    FROM 
        RankedPosts rp
    WHERE 
        rp.rn = 1 AND 
        rp.CreationDate >= CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days'  
)

SELECT 
    p.PostId,
    p.Title,
    p.Tags,
    p.CreationDate,
    p.OwnerDisplayName,
    p.AnswerCount,
    p.UpVotes,
    p.DownVotes,
    COALESCE(ROUND((CAST(p.UpVotes AS FLOAT) / NULLIF((p.UpVotes + p.DownVotes), 0)) * 100, 2), 0) AS UpVotePercentage,
    COALESCE(ROUND((CAST(p.DownVotes AS FLOAT) / NULLIF((p.UpVotes + p.DownVotes), 0)) * 100, 2), 0) AS DownVotePercentage,
    (SELECT STRING_AGG(c.Text, ' | ' ORDER BY c.Id) 
     FROM Comments c 
     WHERE c.PostId = p.PostId) AS CommentSummary
FROM 
    FilteredPosts p
ORDER BY 
    p.UpVotes DESC, p.CreationDate DESC;
