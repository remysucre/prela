
WITH RecentPosts AS (
    SELECT 
        p.Id AS PostId,
        p.Title,
        p.CreationDate,
        p.OwnerUserId,
        p.ParentId,
        p.AcceptedAnswerId,
        COALESCE(Users.DisplayName, 'Community User') AS OwnerDisplayName,
        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) AS rn
    FROM 
        Posts p
    LEFT JOIN 
        Users ON p.OwnerUserId = Users.Id
    WHERE 
        p.CreationDate > (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '30 days')
),
ActiveUsers AS (
    SELECT 
        u.Id AS UserId,
        u.DisplayName,
        COUNT(DISTINCT p.Id) AS PostCount,
        SUM(u.UpVotes) AS TotalUpVotes,
        SUM(u.DownVotes) AS TotalDownVotes,
        RANK() OVER (ORDER BY COUNT(DISTINCT p.Id) DESC) AS Rank
    FROM 
        Users u
    LEFT JOIN 
        Posts p ON u.Id = p.OwnerUserId
    GROUP BY 
        u.Id, u.DisplayName
),
ClosedPosts AS (
    SELECT 
        ph.PostId,
        ph.UserId,
        ph.CreationDate,
        COUNT(CASE WHEN ph.PostHistoryTypeId = 10 THEN 1 END) AS CloseCount
    FROM 
        PostHistory ph
    WHERE 
        ph.PostHistoryTypeId IN (10, 11) 
    GROUP BY 
        ph.PostId, ph.UserId, ph.CreationDate
),
TopClosedPosts AS (
    SELECT 
        cp.PostId,
        p.Title,
        cp.CloseCount,
        COALESCE(u.DisplayName, 'Unknown User') AS CloserUser
    FROM 
        ClosedPosts cp
    JOIN 
        Posts p ON cp.PostId = p.Id
    LEFT JOIN 
        Users u ON cp.UserId = u.Id
    WHERE 
        cp.CloseCount > 0
    ORDER BY 
        cp.CloseCount DESC, cp.PostId, cp.UserId, cp.CreationDate
    LIMIT 10
)
SELECT 
    rp.PostId,
    rp.Title,
    rp.CreationDate,
    rp.OwnerDisplayName,
    au.PostCount AS UserPostCount,
    au.TotalUpVotes,
    au.TotalDownVotes,
    tk.CloseCount AS UserClosedPostCount,
    tk.CloserUser
FROM 
    RecentPosts rp
LEFT JOIN 
    ActiveUsers au ON rp.OwnerUserId = au.UserId
LEFT JOIN 
    TopClosedPosts tk ON rp.PostId = tk.PostId
WHERE 
    (rp.ParentId IS NULL OR rp.AcceptedAnswerId IS NOT NULL)
    AND (au.PostCount > 5 OR au.TotalUpVotes > 100)
ORDER BY 
    rp.CreationDate DESC;
