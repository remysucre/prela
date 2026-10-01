WITH RecursiveCTE AS (
    
    SELECT
        p.Id AS PostId,
        p.Title,
        p.CreationDate,
        p.Score,
        p.OwnerUserId,
        p.AcceptedAnswerId,
        p.ViewCount,
        COALESCE(a.Score, 0) AS AcceptedAnswerScore,
        ROW_NUMBER() OVER (PARTITION BY p.Id ORDER BY p.CreationDate DESC) AS RowNum
    FROM
        Posts p
    LEFT JOIN 
        Posts a ON p.AcceptedAnswerId = a.Id
    WHERE 
        p.PostTypeId = 1
),
UserMetrics AS (
    
    SELECT 
        u.Id AS UserId,
        u.DisplayName,
        COUNT(DISTINCT p.Id) AS TotalPosts,
        SUM(COALESCE(p.Score, 0)) AS TotalScore,
        AVG(COALESCE(p.Score, 0)) AS AverageScore,
        SUM(COALESCE(b.Class, 0)) AS TotalBadges,
        MAX(u.Reputation) AS MaxReputation
    FROM 
        Users u
    LEFT JOIN 
        Posts p ON u.Id = p.OwnerUserId
    LEFT JOIN 
        Badges b ON u.Id = b.UserId
    GROUP BY 
        u.Id, u.DisplayName
)
SELECT 
    p.PostId,
    p.Title,
    p.CreationDate,
    p.ViewCount,
    p.AcceptedAnswerScore,
    um.DisplayName AS Author,
    um.TotalPosts,
    um.TotalScore,
    um.AverageScore,
    CASE 
        WHEN p.AcceptedAnswerId IS NULL THEN 'No Accepted Answer' 
        ELSE 'Accepted Answer Exists' 
    END AS AnswerStatus,
    COUNT(c.Id) AS CommentCount,
    (SELECT COUNT(v.Id) FROM Votes v WHERE v.PostId = p.PostId AND v.VoteTypeId = 2) AS TotalUpVotes,
    (SELECT COUNT(v.Id) FROM Votes v WHERE v.PostId = p.PostId AND v.VoteTypeId = 3) AS TotalDownVotes
FROM 
    RecursiveCTE p
JOIN 
    UserMetrics um ON p.OwnerUserId = um.UserId
LEFT JOIN 
    Comments c ON p.PostId = c.PostId
WHERE 
    p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 MONTH'
GROUP BY 
    p.PostId, p.Title, p.CreationDate, p.ViewCount, p.AcceptedAnswerScore, 
    um.DisplayName, um.TotalPosts, um.TotalScore, um.AverageScore, 
    p.AcceptedAnswerId
ORDER BY 
    p.ViewCount DESC, p.PostId
LIMIT 50;